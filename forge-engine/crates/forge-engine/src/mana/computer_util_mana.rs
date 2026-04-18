use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ManaCostShard, ZoneType};
use indexmap::IndexMap;
use std::collections::HashMap;

use crate::agent::ManaAbilityOption;
use crate::cost::cost_part::pay_cost_from_source;
use crate::cost::{can_pay_ignoring_mana, CostPart};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use crate::parsing::keys;

use super::mana_cost_being_paid::{can_pay_for_shard_with_color, ManaCostBeingPaid};
use super::{
    all_basic_subtype_atoms, atom_short, basic_land_mana_atom, chosen_colors_to_atoms,
    compute_reflected_atoms, fixed_produced_atoms, produced_to_atoms, resolve_mana_ability_amount,
    tap_land_for_mana, ManaPool,
};

#[derive(Debug, Clone)]
struct ManaAbilityRef {
    card_id: CardId,
    ability_index: Option<usize>,
    atoms: Vec<u16>,
    amount: i32,
    mana_text: String,
    source_order: usize,
}

impl ManaAbilityRef {
    fn can_pay_shard(&self, shard: ManaCostShard) -> bool {
        self.atoms
            .iter()
            .any(|&a| can_pay_for_shard_with_color(shard, a))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoTapChoice {
    pub card_id: CardId,
    pub mana_ability_index: Option<usize>,
    pub chosen_atom: u16,
}

#[derive(Debug, Clone)]
pub struct ManaPaymentSources {
    pub source_cards: Vec<CardId>,
    pub mana_ability_options: Vec<ManaAbilityOption>,
}

fn mana_cost_from_cost(cost: &crate::cost::Cost) -> ManaCost {
    let mut out = ManaCost::generic(0);
    for part in &cost.parts {
        if let CostPart::Mana { cost, .. } = part {
            out = out.add(cost);
        }
    }
    out
}

/// Optional callback for choosing which permanent to sacrifice during mana
/// ability cost payment.  When `None`, the engine picks the first target after
/// sorting by (card_name, card_id) — a deterministic fallback.  When `Some`,
/// the callback is invoked with the sorted list of valid targets and should
/// return the chosen card (mirrors Java's `choosePermanentsToSacrifice`
/// which uses the harness RNG).
pub type SacrificeChooser<'a> = &'a mut dyn FnMut(&[CardId]) -> Option<CardId>;

/// Callback parameter for mana ability payment decisions.
/// Used to dispatch both sacrifice chooser and confirm payment callbacks
/// through a single unified interface to avoid multiple mutable borrows.
#[derive(Debug)]
pub enum ManaPayCallback<'a> {
    /// Choose which permanent to sacrifice from the given list.
    /// Return the chosen card, or None to cancel.
    ChooseSacrifice(&'a [CardId]),
    /// Notify the caller that auto-pay is making a color-choice prompt.
    /// The callback may use this to preserve parity-visible prompt ordering.
    /// The return value is ignored for this variant.
    ChooseColor(&'a [String]),
    /// Confirm whether to sacrifice the given card for a mana ability.
    /// Return true to proceed, false to cancel.
    /// Mirrors Java's DeterministicCostDecision.confirmPayment() path.
    ConfirmSelfSacrifice(CardId),
    /// Confirm whether to remove counters from the source for a mana ability.
    /// Mirrors Java CostPayment confirm for CostRemoveCounter (SubCounter).
    ConfirmSubCounter(CardId),
    /// Confirm whether to exile the source for a mana ability.
    /// Mirrors Java CostPayment confirm for source-paid CostExile.
    ConfirmSourceExile(CardId),
    /// Execute the sacrifice of the given permanent for a mana ability.
    /// The callback is responsible for firing Sacrificed/ChangesZone using
    /// battlefield LKI, moving the card, and returning the same card id on
    /// success. Returning `None` cancels the payment.
    NotifySacrificeForMana(CardId),
}

/// Unified callback for mana payment decisions during auto-tap.
/// Returns Some(card_id) on success, None to cancel.
pub type ManaPayCallbackFn<'a> = &'a mut dyn FnMut(ManaPayCallback<'_>) -> Option<CardId>;

/// Auto-tap lands to produce the required mana.
/// Mirrors harness AutoPay flow used by parity tests: collect currently playable
/// mana abilities in battlefield order, choose the first legal source for the
/// next unpaid shard, then repeat after each activation.
pub fn auto_tap_lands(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<CardId> {
    auto_tap_lands_trace(game, pool, player, cost, current_spell)
        .into_iter()
        .map(|choice| choice.card_id)
        .collect()
}

pub fn auto_tap_lands_allow_reserved_source_reuse(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<CardId> {
    auto_tap_lands_allow_reserved_source_reuse_trace(game, pool, player, cost, current_spell)
        .into_iter()
        .map(|choice| choice.card_id)
        .collect()
}

pub fn auto_tap_lands_trace(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<AutoTapChoice> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        false,
        &[],
        &mut None,
    )
}

pub fn auto_tap_lands_allow_reserved_source_reuse_trace(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<AutoTapChoice> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        true,
        &[],
        &mut None,
    )
}

/// Auto-tap with an explicit sacrifice chooser callback for parity with Java's
/// `choosePermanentsToSacrifice` RNG path.
pub fn auto_tap_lands_with_chooser(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    sacrifice_chooser: SacrificeChooser<'_>,
) -> Vec<CardId> {
    let mut callback = |kind: ManaPayCallback<'_>| -> Option<CardId> {
        match kind {
            ManaPayCallback::ChooseSacrifice(valid) => sacrifice_chooser(valid),
            ManaPayCallback::ChooseColor(_) => None,
            ManaPayCallback::ConfirmSelfSacrifice(cid) => Some(cid),
            ManaPayCallback::ConfirmSubCounter(cid) => Some(cid),
            ManaPayCallback::ConfirmSourceExile(cid) => Some(cid),
            ManaPayCallback::NotifySacrificeForMana(cid) => Some(cid),
        }
    };
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        false,
        &[],
        &mut Some(&mut callback),
    )
    .into_iter()
    .map(|choice| choice.card_id)
    .collect()
}

pub fn auto_tap_lands_allow_reserved_source_reuse_with_chooser(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    sacrifice_chooser: SacrificeChooser<'_>,
) -> Vec<CardId> {
    let mut callback = |kind: ManaPayCallback<'_>| -> Option<CardId> {
        match kind {
            ManaPayCallback::ChooseSacrifice(valid) => sacrifice_chooser(valid),
            ManaPayCallback::ChooseColor(_) => None,
            ManaPayCallback::ConfirmSelfSacrifice(cid) => Some(cid),
            ManaPayCallback::ConfirmSubCounter(cid) => Some(cid),
            ManaPayCallback::ConfirmSourceExile(cid) => Some(cid),
            ManaPayCallback::NotifySacrificeForMana(cid) => Some(cid),
        }
    };
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        true,
        &[],
        &mut Some(&mut callback),
    )
    .into_iter()
    .map(|choice| choice.card_id)
    .collect()
}

/// Auto-tap with unified callback for both sacrifice chooser and confirm payment.
/// Used by parity tests to mirror Java's RNG-driven decision paths.
pub fn auto_tap_lands_with_callbacks(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    callback: ManaPayCallbackFn<'_>,
) -> Vec<CardId> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        false,
        &[],
        &mut Some(callback),
    )
    .into_iter()
    .map(|choice| choice.card_id)
    .collect()
}

pub fn auto_tap_lands_trace_with_callbacks(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    callback: ManaPayCallbackFn<'_>,
) -> Vec<AutoTapChoice> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        false,
        &[],
        &mut Some(callback),
    )
}

/// Determine the next mana source/ability auto-pay would use without mutating
/// the game or pool. This lets callback-driven payment replay the exact same
/// source choice as engine auto-pay, including multi-ability lands.
pub fn next_auto_tap_choice(
    game: &GameState,
    pool: &ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    allow_reserved_source_reuse: bool,
) -> Option<AutoTapChoice> {
    next_auto_tap_choice_with_reserved_sacrifices(
        game,
        pool,
        player,
        cost,
        current_spell,
        allow_reserved_source_reuse,
        &[],
    )
}

pub fn next_auto_tap_choice_with_reserved_sacrifices(
    game: &GameState,
    pool: &ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> Option<AutoTapChoice> {
    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, pool);
    if unpaid.is_paid() {
        return None;
    }

    let mana_ability_map = group_sources_by_mana_color(game, player, reserved_sacrifices, None);
    if mana_ability_map.is_empty() {
        return None;
    }

    let mut sources_for_shards = group_and_order_to_pay_shards(&mana_ability_map, &unpaid);
    if sources_for_shards.is_empty() {
        return None;
    }
    sort_sources_for_autopay(game, player, &mut sources_for_shards);

    let to_pay = get_next_shard_to_pay(&unpaid, &sources_for_shards)?;
    let ma_list = sources_for_shards.get(&to_pay)?;
    let sa_payment = choose_mana_ability(
        game,
        player,
        current_spell,
        to_pay,
        ma_list,
        allow_reserved_source_reuse,
        reserved_sacrifices,
        &sources_for_shards,
        &unpaid,
    )?;
    let chosen_atom = choose_atom_for_shard(&sa_payment, to_pay)?;
    Some(AutoTapChoice {
        card_id: sa_payment.card_id,
        mana_ability_index: sa_payment.ability_index,
        chosen_atom,
    })
}

pub fn auto_tap_lands_allow_reserved_source_reuse_with_callbacks(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    callback: ManaPayCallbackFn<'_>,
) -> Vec<CardId> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        true,
        &[],
        &mut Some(callback),
    )
    .into_iter()
    .map(|choice| choice.card_id)
    .collect()
}

pub fn auto_tap_lands_allow_reserved_source_reuse_trace_with_callbacks_and_reserved_sacrifices(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    reserved_sacrifices: &[CardId],
    callback: ManaPayCallbackFn<'_>,
) -> Vec<AutoTapChoice> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        true,
        reserved_sacrifices,
        &mut Some(callback),
    )
}

pub fn auto_tap_lands_allow_reserved_source_reuse_with_callbacks_and_reserved_sacrifices(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    reserved_sacrifices: &[CardId],
    callback: ManaPayCallbackFn<'_>,
) -> Vec<CardId> {
    auto_tap_lands_internal(
        game,
        pool,
        player,
        cost,
        current_spell,
        true,
        reserved_sacrifices,
        &mut Some(callback),
    )
    .into_iter()
    .map(|choice| choice.card_id)
    .collect()
}

/// Mirrors Java AutoPay.payManaCost() — the main auto-tap loop.
///
/// Key parity points:
/// - Re-collects candidates EVERY iteration (fresh source list after each tap/sacrifice)
/// - Tries ALL shards in priority order per iteration via `choose_candidate`
/// - Uses `is_sole_source_for_other_shard` to preserve flexible sources
/// - Delegates sacrifice/counter costs through the callback
fn auto_tap_lands_internal(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
    callback: &mut Option<ManaPayCallbackFn<'_>>,
) -> Vec<AutoTapChoice> {
    let mut tapped_choices: Vec<AutoTapChoice> = Vec::new();

    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, pool);
    if unpaid.is_paid() {
        return tapped_choices;
    }

    // Guard counter mirrors Java's AutoPay.payManaCost() `guard++ < 128`.
    let mut guard = 0u32;
    while !unpaid.is_paid() && guard < 128 {
        guard += 1;

        // Java re-collects candidates every iteration. This ensures tapped/sacrificed
        // sources are excluded and state changes from the previous iteration are visible.
        let mana_ability_map = group_sources_by_mana_color(game, player, reserved_sacrifices, None);
        if mana_ability_map.is_empty() {
            break;
        }
        let mut candidates = collect_sorted_candidates(game, player, &mana_ability_map);
        if candidates.is_empty() {
            break;
        }

        // Java's chooseCandidate: iterate shards in priority order, pick the
        // least-versatile candidate that can pay.
        let Some((sa_payment, to_pay)) = choose_candidate(
            game,
            player,
            current_spell,
            &candidates,
            &unpaid,
            allow_reserved_source_reuse,
            reserved_sacrifices,
        ) else {
            break;
        };

        let Some(chosen_atom) = choose_atom_for_shard(&sa_payment, to_pay) else {
            break;
        };

        // Pay non-tap ability costs (sacrifice, counter removal) through callback.
        // If payment fails (e.g. sacrifice declined), remove the candidate and retry.
        if !pay_non_tap_mana_ability_costs(
            game,
            player,
            &sa_payment,
            current_spell,
            allow_reserved_source_reuse,
            reserved_sacrifices,
            callback,
        ) {
            // Java: candidate became unpayable; remove and continue.
            candidates.retain(|c| c.card_id != sa_payment.card_id);
            continue;
        }

        if let Some(fixed_atoms) = fixed_produced_atoms(&sa_payment.mana_text, &[]) {
            let pain = super::land_pain_damage(game.card(sa_payment.card_id), chosen_atom);
            let is_snow = game.card(sa_payment.card_id).type_line.is_snow();
            if source_requires_tap(game, &sa_payment) && !game.card(sa_payment.card_id).tapped {
                game.tap(sa_payment.card_id);
            }
            let repeats = (sa_payment.amount.max(1) as usize)
                .checked_div(fixed_atoms.len().max(1))
                .unwrap_or(1)
                .max(1);
            for _ in 0..repeats {
                for &atom in &fixed_atoms {
                    if is_snow {
                        pool.add_snow(atom, 1);
                    } else {
                        pool.add(atom, 1);
                    }
                    let _ = unpaid.try_pay_mana(atom, atom as u8);
                }
            }
            if pain > 0 {
                game.player_lose_life(player, pain);
            }
            tapped_choices.push(AutoTapChoice {
                card_id: sa_payment.card_id,
                mana_ability_index: sa_payment.ability_index,
                chosen_atom,
            });
        } else {
            if let Some(ref mut cb) = callback {
                let chosen_colors = game.card(sa_payment.card_id).chosen_colors.clone();
                let valid_colors =
                    super::produced_to_color_names(&sa_payment.mana_text, &chosen_colors);
                if valid_colors.len() > 1 {
                    if let Some(color_name) = super::mana_atom_to_color_name(chosen_atom) {
                        let forced = [color_name.to_string()];
                        cb(ManaPayCallback::ChooseColor(&forced));
                    }
                }
            }
            tap_land_for_mana(
                game,
                pool,
                player,
                sa_payment.card_id,
                chosen_atom,
                source_requires_tap(game, &sa_payment),
                &mut Vec::new(),
            );

            tapped_choices.push(AutoTapChoice {
                card_id: sa_payment.card_id,
                mana_ability_index: sa_payment.ability_index,
                chosen_atom,
            });

            let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
            for _ in 1..sa_payment.amount.max(1) {
                pool.add(chosen_atom, 1);
                let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
            }
        }
    }

    tapped_choices
}

fn pay_cost_from_pool(unpaid: &mut ManaCostBeingPaid, pool: &ManaPool) {
    let colors = [
        (ManaAtom::WHITE, pool.white()),
        (ManaAtom::BLUE, pool.blue()),
        (ManaAtom::BLACK, pool.black()),
        (ManaAtom::RED, pool.red()),
        (ManaAtom::GREEN, pool.green()),
        (ManaAtom::COLORLESS, pool.colorless()),
    ];

    for (atom, count) in colors {
        for _ in 0..count.max(0) {
            if unpaid.is_paid() {
                return;
            }
            let _ = unpaid.try_pay_mana(atom, atom as u8);
        }
    }
}

fn get_next_shard_to_pay(
    unpaid: &ManaCostBeingPaid,
    sources_for_shards: &IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
) -> Option<ManaCostShard> {
    let mut shards_to_pay = unpaid.get_distinct_shards();
    shards_to_pay.sort_by_key(|shard| sources_for_shards.get(shard).map_or(0, |v| v.len()));
    unpaid.get_shard_to_pay_by_priority(&shards_to_pay, ManaAtom::COLORS_SUPERPOSITION as u8)
}

/// Build a flat, sorted candidate list from the mana ability map.
/// Mirrors Java AutoPay.collectPlayableManaAbilities() — called fresh each iteration.
fn collect_sorted_candidates(
    game: &GameState,
    player: PlayerId,
    mana_ability_map: &IndexMap<i32, Vec<ManaAbilityRef>>,
) -> Vec<ManaAbilityRef> {
    let mut out: Vec<ManaAbilityRef> = mana_ability_map
        .values()
        .flat_map(|v| v.iter().cloned())
        .collect();
    // Deduplicate by (card_id, ability_index) — same ability may appear under multiple color keys.
    let mut seen = std::collections::HashSet::new();
    out.retain(|ma| seen.insert((ma.card_id, ma.ability_index, ma.source_order)));
    // Sort by score (lands before creatures), matching Java's Comparator.comparingInt(score).
    out.sort_by_key(|ma| autopay_source_score(game, player, ma) * 1000 + ma.source_order as i32);
    out
}

/// Mirrors Java AutoPay.chooseCandidate() — iterate shards in priority order,
/// pick the least-versatile candidate that can pay.
///
/// Returns the chosen source and the shard it will pay.
fn choose_candidate(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    candidates: &[ManaAbilityRef],
    unpaid: &ManaCostBeingPaid,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> Option<(ManaAbilityRef, ManaCostShard)> {
    for shard in shard_priority(unpaid, candidates) {
        if let Some(ma) = choose_least_versatile_candidate(
            game,
            player,
            current_spell,
            candidates,
            shard,
            unpaid,
            allow_reserved_source_reuse,
            reserved_sacrifices,
        ) {
            return Some((ma, shard));
        }
    }
    None
}

/// Mirrors Java AutoPay.shardPriority() — colored shards sorted by fewest
/// available candidates (most constrained first), then generic. X is skipped.
fn shard_priority(unpaid: &ManaCostBeingPaid, candidates: &[ManaAbilityRef]) -> Vec<ManaCostShard> {
    let mut colored = Vec::new();
    let mut generic = None;
    let mut seen = std::collections::HashSet::new();
    for shard in unpaid.get_distinct_shards() {
        if matches!(shard, ManaCostShard::X | ManaCostShard::ColoredX) {
            continue;
        }
        if !seen.insert(shard) {
            continue;
        }
        if matches!(shard, ManaCostShard::Generic) {
            generic = Some(shard);
        } else {
            colored.push(shard);
        }
    }
    colored.sort_by_key(|&shard| count_candidates_for_shard(candidates, shard));
    if let Some(g) = generic {
        colored.push(g);
    }
    colored
}

/// Count how many candidates can pay a given shard.
fn count_candidates_for_shard(candidates: &[ManaAbilityRef], shard: ManaCostShard) -> usize {
    candidates.iter().filter(|c| c.can_pay_shard(shard)).count()
}

/// Mirrors Java AutoPay.chooseLeastVersatileCandidate() — pick the first
/// candidate that can pay the shard, but defer candidates that are the sole
/// source for another unpaid colored shard. Falls back if all are sole sources.
fn choose_least_versatile_candidate(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    candidates: &[ManaAbilityRef],
    shard: ManaCostShard,
    unpaid: &ManaCostBeingPaid,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> Option<ManaAbilityRef> {
    let mut fallback: Option<ManaAbilityRef> = None;
    for ma in candidates {
        if Some(ma.card_id) == current_spell {
            continue;
        }
        if !ma.can_pay_shard(shard) {
            continue;
        }
        if !can_pay_non_tap_mana_ability_costs(
            game,
            player,
            ma,
            current_spell,
            allow_reserved_source_reuse,
            reserved_sacrifices,
        ) {
            continue;
        }
        if fallback.is_none() {
            fallback = Some(ma.clone());
        }
        if !is_sole_source_for_other_shard_candidates(ma, shard, candidates, unpaid) {
            return Some(ma.clone());
        }
    }
    fallback
}

/// Mirrors Java AutoPay.isSoleSourceForOtherShard() using the flat candidate list.
fn is_sole_source_for_other_shard_candidates(
    candidate: &ManaAbilityRef,
    current_shard: ManaCostShard,
    candidates: &[ManaAbilityRef],
    unpaid: &ManaCostBeingPaid,
) -> bool {
    let mut seen = std::collections::HashSet::new();
    for other_shard in unpaid.get_distinct_shards() {
        if other_shard == current_shard {
            continue;
        }
        if matches!(
            other_shard,
            ManaCostShard::Generic | ManaCostShard::X | ManaCostShard::ColoredX
        ) {
            continue;
        }
        if !seen.insert(other_shard) {
            continue;
        }
        if !candidate.can_pay_shard(other_shard) {
            continue;
        }
        let sources_for_other = candidates
            .iter()
            .filter(|alt| alt.can_pay_shard(other_shard))
            .count();
        if sources_for_other <= 1 {
            return true;
        }
    }
    false
}

/// Choose the best mana ability to pay a shard, mirroring Java AutoPay's
/// `chooseLeastVersatileCandidate()`.
///
/// Defers choosing a source if it is the sole provider for another unpaid
/// colored shard — this preserves flexible dual lands for the shards that
/// truly need them. Falls back to the first valid candidate if every
/// candidate is a sole source for something.
fn choose_mana_ability(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    to_pay: ManaCostShard,
    ma_list: &[ManaAbilityRef],
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
    sources_for_shards: &IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
    unpaid: &ManaCostBeingPaid,
) -> Option<ManaAbilityRef> {
    let mut fallback: Option<ManaAbilityRef> = None;

    for ma in ma_list {
        if Some(ma.card_id) == current_spell {
            continue;
        }
        if !ma.can_pay_shard(to_pay)
            || !can_pay_non_tap_mana_ability_costs(
                game,
                player,
                ma,
                current_spell,
                allow_reserved_source_reuse,
                reserved_sacrifices,
            )
        {
            continue;
        }

        if fallback.is_none() {
            fallback = Some(ma.clone());
        }

        // Check if this candidate is the sole source for another unpaid shard.
        // If so, defer it — another shard needs it more.
        if !is_sole_source_for_other_shard(ma, to_pay, sources_for_shards, unpaid) {
            return Some(ma.clone());
        }
    }

    // All valid candidates are sole sources for other shards.
    // Fall back to the first valid one (forced pick).
    fallback
}

/// Mirrors Java AutoPay's `isSoleSourceForOtherShard()`.
///
/// Returns true if `candidate` is the ONLY source that can pay for some
/// other unpaid colored shard (not the current one, not generic/X).
fn is_sole_source_for_other_shard(
    candidate: &ManaAbilityRef,
    current_shard: ManaCostShard,
    sources_for_shards: &IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
    unpaid: &ManaCostBeingPaid,
) -> bool {
    for other_shard in unpaid.get_distinct_shards() {
        if other_shard == current_shard {
            continue;
        }
        // Skip generic/X shards — they can be paid by anything.
        if matches!(
            other_shard,
            ManaCostShard::Generic | ManaCostShard::X | ManaCostShard::ColoredX
        ) {
            continue;
        }
        if !candidate.can_pay_shard(other_shard) {
            continue;
        }
        // Count how many sources in the pool can pay for this other shard.
        let sources_for_other = sources_for_shards
            .get(&other_shard)
            .map(|list| {
                list.iter()
                    .filter(|alt| alt.can_pay_shard(other_shard))
                    .count()
            })
            .unwrap_or(0);
        if sources_for_other <= 1 {
            return true; // This candidate is the only source — defer it.
        }
    }
    false
}

fn can_pay_non_tap_mana_ability_costs(
    game: &GameState,
    player: PlayerId,
    ma: &ManaAbilityRef,
    reserved_source: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> bool {
    let Some(ab_idx) = ma.ability_index else {
        return true;
    };
    let cost_parts: Vec<_> = game.card(ma.card_id).activated_abilities[ab_idx]
        .cost
        .parts
        .clone();
    for part in &cost_parts {
        if !can_pay_source_paid_mana_cost_part(
            game,
            player,
            ma.card_id,
            part,
            reserved_source,
            allow_reserved_source_reuse,
            reserved_sacrifices,
        ) {
            return false;
        }
    }
    true
}

fn pay_non_tap_mana_ability_costs(
    game: &mut GameState,
    player: PlayerId,
    ma: &ManaAbilityRef,
    reserved_source: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
    callback: &mut Option<ManaPayCallbackFn<'_>>,
) -> bool {
    let Some(ab_idx) = ma.ability_index else {
        return true;
    };
    let cost_parts: Vec<_> = game.card(ma.card_id).activated_abilities[ab_idx]
        .cost
        .parts
        .clone();
    for part in &cost_parts {
        match part {
            CostPart::Tap | CostPart::Mana { .. } => {}
            CostPart::PayLife(amount) => {
                if game.player(player).life < *amount {
                    return false;
                }
                game.player_lose_life(player, *amount);
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                if game.card(ma.card_id).counter_count(counter_type) < *amount {
                    return false;
                }
                if let Some(ref mut cb) = callback {
                    if let Some(confirmed_id) = cb(ManaPayCallback::ConfirmSubCounter(ma.card_id)) {
                        if confirmed_id != ma.card_id {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                game.card_mut(ma.card_id)
                    .remove_counter(counter_type, *amount);
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if *amount > 1 || game.card(ma.card_id).zone != ZoneType::Battlefield {
                        return false;
                    }
                    // Java parity: AutoPay.payAbilityActivationCosts() delegates to
                    // costPlumbing.payWithDeterministicDecision() which calls
                    // DeterministicCostDecision.visit(CostSacrifice) → confirm()
                    // → confirmPayment(). This consumes RNG for non-spell contexts.
                    if let Some(ref mut cb) = callback {
                        if let Some(confirmed_id) =
                            cb(ManaPayCallback::ConfirmSelfSacrifice(ma.card_id))
                        {
                            if confirmed_id != ma.card_id {
                                return false;
                            }
                        } else {
                            return false; // confirmation declined
                        }
                    }
                    if let Some(ref mut cb) = callback {
                        if let Some(sacrificed_id) =
                            cb(ManaPayCallback::NotifySacrificeForMana(ma.card_id))
                        {
                            if sacrificed_id != ma.card_id {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let owner = game.card(ma.card_id).owner;
                        game.move_card(ma.card_id, ZoneType::Graveyard, owner);
                    }
                } else {
                    let mut targets = crate::cost::get_sacrifice_targets_for_cost(
                        game,
                        player,
                        type_filter,
                        None,
                    );
                    targets.retain(|cid| !reserved_sacrifices.contains(cid));
                    if !allow_reserved_source_reuse {
                        if let Some(reserved) = reserved_source {
                            targets.retain(|&cid| cid != reserved);
                        }
                    }
                    targets.sort_by(|&a, &b| {
                        game.card(a)
                            .card_name
                            .cmp(&game.card(b).card_name)
                            .then_with(|| a.index().cmp(&b.index()))
                    });
                    let required = (*amount).max(0) as usize;
                    if targets.len() < required {
                        return false;
                    }
                    // Use the sacrifice chooser callback when available (parity
                    // with Java's choosePermanentsToSacrifice which uses RNG).
                    // Otherwise fall back to deterministic first-by-index.
                    for _ in 0..required {
                        let chosen = if let Some(ref mut cb) = callback {
                            cb(ManaPayCallback::ChooseSacrifice(&targets))
                        } else {
                            targets.first().copied()
                        };
                        if let Some(cid) = chosen {
                            targets.retain(|&c| c != cid);
                            if let Some(ref mut cb) = callback {
                                if let Some(sacrificed_id) =
                                    cb(ManaPayCallback::NotifySacrificeForMana(cid))
                                {
                                    if sacrificed_id != cid {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            } else {
                                let owner = game.card(cid).owner;
                                game.move_card(cid, ZoneType::Graveyard, owner);
                            }
                        }
                    }
                }
            }
            CostPart::Exile { amount, from, .. } => {
                if !pay_cost_from_source(part) || *amount > 1 || game.card(ma.card_id).zone != *from
                {
                    return false;
                }
                if let Some(ref mut cb) = callback {
                    if let Some(confirmed_id) = cb(ManaPayCallback::ConfirmSourceExile(ma.card_id))
                    {
                        if confirmed_id != ma.card_id {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                let owner = game.card(ma.card_id).owner;
                game.move_card(ma.card_id, ZoneType::Exile, owner);
            }
            _ => return false,
        }
    }
    true
}

fn can_pay_source_paid_mana_cost_part(
    game: &GameState,
    player: PlayerId,
    source_id: CardId,
    part: &CostPart,
    reserved_source: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> bool {
    match part {
        CostPart::Tap | CostPart::Mana { .. } => true,
        CostPart::PayLife(amount) => game.player(player).life >= *amount,
        CostPart::SubCounter {
            amount,
            counter_type,
        } => game.card(source_id).counter_count(counter_type) >= *amount,
        CostPart::Sacrifice {
            type_filter,
            amount,
        } => {
            if type_filter == "CARDNAME" {
                *amount <= 1
                    && game.card(source_id).zone == ZoneType::Battlefield
                    && !reserved_sacrifices.contains(&source_id)
            } else {
                let targets = get_payable_mana_sacrifice_targets(
                    game,
                    player,
                    type_filter,
                    reserved_source,
                    allow_reserved_source_reuse,
                    reserved_sacrifices,
                );
                (targets.len() as i32) >= *amount
            }
        }
        CostPart::Exile { amount, from, .. } => {
            pay_cost_from_source(part) && *amount <= 1 && game.card(source_id).zone == *from
        }
        _ => false,
    }
}

fn get_payable_mana_sacrifice_targets(
    game: &GameState,
    player: PlayerId,
    type_filter: &str,
    reserved_source: Option<CardId>,
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
) -> Vec<CardId> {
    let mut targets = crate::cost::get_sacrifice_targets_for_cost(game, player, type_filter, None);
    targets.retain(|cid| !reserved_sacrifices.contains(cid));
    if !allow_reserved_source_reuse {
        if let Some(reserved) = reserved_source {
            targets.retain(|&cid| cid != reserved);
        }
    }
    targets
}

fn choose_atom_for_shard(mana_ab: &ManaAbilityRef, shard: ManaCostShard) -> Option<u16> {
    if shard.is_colorless() {
        if mana_ab.atoms.contains(&ManaAtom::COLORLESS) {
            return Some(ManaAtom::COLORLESS);
        }
    }

    if shard == ManaCostShard::Generic || shard.is_generic() {
        return mana_ab.atoms.first().copied();
    }

    mana_ab
        .atoms
        .iter()
        .copied()
        .find(|&a| can_pay_for_shard_with_color(shard, a))
}

fn group_and_order_to_pay_shards(
    mana_ability_map: &IndexMap<i32, Vec<ManaAbilityRef>>,
    cost: &ManaCostBeingPaid,
) -> IndexMap<ManaCostShard, Vec<ManaAbilityRef>> {
    let mut res: IndexMap<ManaCostShard, Vec<ManaAbilityRef>> = IndexMap::new();

    if (cost.get_generic_mana_amount() > 0 || cost.has_any_kind(ManaAtom::OR_2_GENERIC))
        && mana_ability_map.contains_key(&(ManaAtom::GENERIC as i32))
    {
        res.insert(
            ManaCostShard::Generic,
            mana_ability_map
                .get(&(ManaAtom::GENERIC as i32))
                .cloned()
                .unwrap_or_default(),
        );
    }

    for shard in cost.get_distinct_shards() {
        if shard.is_or_2_generic() {
            let color_key = shard.color_mask() as i32;
            if let Some(list) = mana_ability_map.get(&color_key) {
                res.entry(shard).or_default().extend(list.clone());
            }
            if let Some(list) = mana_ability_map.get(&(ManaAtom::GENERIC as i32)) {
                res.entry(shard).or_default().extend(list.clone());
            }
            continue;
        }

        if shard == ManaCostShard::Generic {
            continue;
        }

        for (color_key, list) in mana_ability_map {
            let key_color =
                (*color_key as u16) & (ManaAtom::COLORS_SUPERPOSITION | ManaAtom::COLORLESS);
            if can_pay_for_shard_with_color(shard, key_color) {
                let bucket = res.entry(shard).or_default();
                for ma in list {
                    if !bucket
                        .iter()
                        .any(|x| x.card_id == ma.card_id && x.ability_index == ma.ability_index)
                    {
                        bucket.push(ma.clone());
                    }
                }
            }
        }
    }

    res
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn sort_mana_abilities(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    mana_ability_map: &mut IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
    colors_most_common: &[u16],
) {
    let mut mana_card_score: HashMap<CardId, i32> = HashMap::new();
    let mut ordered_cards: Vec<CardId> = Vec::new();

    for abilities in mana_ability_map.values() {
        for ability in abilities {
            if mana_card_score.contains_key(&ability.card_id) {
                continue;
            }
            let score = score_mana_producing_card(game, ability.card_id, player);
            mana_card_score.insert(ability.card_id, score);
            ordered_cards.push(ability.card_id);
        }
    }

    ordered_cards.sort_by_key(|cid| mana_card_score.get(cid).copied().unwrap_or(0));

    let shards: Vec<ManaCostShard> = mana_ability_map.keys().copied().collect();
    for shard in shards {
        let Some(existing) = mana_ability_map.get(&shard).cloned() else {
            continue;
        };
        let mut new_abilities = existing.clone();
        let existing_index: HashMap<(CardId, Option<usize>), usize> = existing
            .iter()
            .enumerate()
            .map(|(i, a)| ((a.card_id, a.ability_index), i))
            .collect();

        // Use binary insertion sort to match Java's TimSort behaviour for small
        // arrays.  Java's TimSort delegates to binary insertion sort for runs
        // shorter than ~32 elements.  Because the comparator is non-transitive,
        // different sort algorithms can (and do) produce different orderings.
        // Rust's `slice::sort_by` uses a merge-sort variant that disagrees with
        // Java on certain inputs, so we replicate the exact insertion-sort loop
        // that Java executes.
        let cmp = |a: &ManaAbilityRef, b: &ManaAbilityRef| -> std::cmp::Ordering {
            let idx_a = ordered_cards
                .iter()
                .position(|&c| c == a.card_id)
                .unwrap_or(usize::MAX);
            let idx_b = ordered_cards
                .iter()
                .position(|&c| c == b.card_id)
                .unwrap_or(usize::MAX);
            let mut pre_order = (idx_a as isize) - (idx_b as isize);

            if pre_order != 0 {
                if shard.is_generic()
                    && mana_card_score.get(&a.card_id) == mana_card_score.get(&b.card_id)
                {
                    for &col in colors_most_common {
                        let a_can = a.atoms.contains(&col);
                        let b_can = b.atoms.contains(&col);
                        if a_can && !b_can {
                            return std::cmp::Ordering::Greater;
                        }
                        if !a_can && b_can {
                            return std::cmp::Ordering::Less;
                        }
                    }
                }

                let a_pos = existing_index
                    .get(&(a.card_id, a.ability_index))
                    .copied()
                    .unwrap_or(usize::MAX);
                let b_pos = existing_index
                    .get(&(b.card_id, b.ability_index))
                    .copied()
                    .unwrap_or(usize::MAX);
                pre_order += (a_pos as isize) - (b_pos as isize);

                return pre_order.cmp(&0);
            }

            let shard_mana = shard.short_string();
            let pay_with_a = a.mana_text.contains(shard_mana);
            let pay_with_b = b.mana_text.contains(shard_mana);
            if pay_with_a && !pay_with_b {
                return std::cmp::Ordering::Less;
            }
            if pay_with_b && !pay_with_a {
                return std::cmp::Ordering::Greater;
            }

            a.ability_index
                .cmp(&b.ability_index)
                .then(a.source_order.cmp(&b.source_order))
        };
        // Java-compatible binary insertion sort (mirrors TimSort's binarySort).
        // For each element at position `i`, binary-search the sorted prefix
        // [0..i) to find where it belongs, then shift elements right and insert.
        for i in 1..new_abilities.len() {
            let pivot = new_abilities[i].clone();
            // Binary search: find leftmost position where pivot should go.
            let mut lo = 0usize;
            let mut hi = i;
            while lo < hi {
                let mid = (lo + hi) / 2;
                // Java: if (c.compare(pivot, a[mid]) < 0) hi = mid; else lo = mid+1;
                if cmp(&pivot, &new_abilities[mid]).is_lt() {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            // Shift [lo..i) right by one, then place pivot at lo.
            if lo < i {
                for j in (lo..i).rev() {
                    new_abilities.swap(j, j + 1);
                }
                new_abilities[lo] = pivot;
            }
        }

        // Java excludes same-host payment in chooseManaAbility, keep list intact here.
        let _ = current_spell;
        mana_ability_map.insert(shard, new_abilities);
    }
}

fn group_sources_by_mana_color(
    game: &GameState,
    player: PlayerId,
    reserved_sacrifices: &[CardId],
    payment_ctx: Option<&crate::mana::ManaPaymentContext>,
) -> IndexMap<i32, Vec<ManaAbilityRef>> {
    let mut mana_map: IndexMap<i32, Vec<ManaAbilityRef>> = IndexMap::new();
    let mut source_order = 0usize;

    for card_id in get_available_mana_sources(game, player, reserved_sacrifices) {
        let card = game.card(card_id);
        let mut explicit_mana_added = false;

        for ab in &card.activated_abilities {
            if !is_payable_mana_ability(game, player, card_id, ab, reserved_sacrifices, payment_ctx)
            {
                continue;
            }
            // Handle ManaReflected abilities (e.g. Incubation Druid)
            if ab.params.get(keys::AB) == Some("ManaReflected") {
                let reflected_atoms = compute_reflected_atoms(game, player, card_id, ab);
                if !reflected_atoms.is_empty() {
                    explicit_mana_added = true;
                    let ma = ManaAbilityRef {
                        card_id,
                        ability_index: Some(ab.ability_index),
                        atoms: reflected_atoms,
                        amount: parse_mana_ability_amount_with_game(
                            ab,
                            Some(game),
                            Some(card_id),
                            Some(player),
                        ),
                        mana_text: "Reflected".to_string(),
                        source_order,
                    };
                    source_order += 1;
                    add_mana_ability_to_color_map(&mut mana_map, &ma);
                }
                continue;
            }

            let Some(produced) = ab.params.get(keys::PRODUCED) else {
                continue;
            };
            let atoms = if produced == "Combo ColorIdentity" {
                let colors = game.player_commander_color_identity(player);
                if colors.is_empty() {
                    Vec::new()
                } else {
                    chosen_colors_to_atoms(&colors)
                }
            } else {
                produced_to_atoms(produced, &card.chosen_colors)
            };
            if atoms.is_empty() {
                continue;
            }

            explicit_mana_added = true;
            let fixed_output_multiplier = fixed_produced_atoms(produced, &card.chosen_colors)
                .map(|atoms| atoms.len() as i32)
                .unwrap_or(1);
            let replacement_multiplier = atoms
                .iter()
                .map(|&atom| {
                    super::replacement_adjusted_atoms_for_availability(game, player, card_id, atom)
                        .len() as i32
                })
                .max()
                .unwrap_or(1)
                .max(1);
            let ma = ManaAbilityRef {
                card_id,
                ability_index: Some(ab.ability_index),
                atoms: atoms.clone(),
                amount: parse_mana_ability_amount_with_game(
                    ab,
                    Some(game),
                    Some(card_id),
                    Some(player),
                ) * fixed_output_multiplier
                    * replacement_multiplier,
                mana_text: produced.to_string(),
                source_order,
            };
            source_order += 1;
            add_mana_ability_to_color_map(&mut mana_map, &ma);
        }

        if !explicit_mana_added {
            if card.zone == ZoneType::Battlefield && card.is_land() && !card.tapped {
                let mut atoms = all_basic_subtype_atoms(card);
                if atoms.is_empty() {
                    if let Some(a) = basic_land_mana_atom(card) {
                        atoms.push(a);
                    }
                }
                for atom in atoms {
                    let replacement_multiplier = super::replacement_adjusted_atoms_for_availability(
                        game, player, card_id, atom,
                    )
                    .len() as i32;
                    let ma = ManaAbilityRef {
                        card_id,
                        ability_index: None,
                        atoms: vec![atom],
                        amount: replacement_multiplier.max(1),
                        mana_text: atom_short(atom).to_string(),
                        source_order,
                    };
                    source_order += 1;
                    add_mana_ability_to_color_map(&mut mana_map, &ma);
                }
            }
        }
    }

    mana_map
}

fn add_mana_ability_to_color_map(
    map: &mut IndexMap<i32, Vec<ManaAbilityRef>>,
    ma: &ManaAbilityRef,
) {
    map.entry(ManaAtom::GENERIC as i32)
        .or_default()
        .push(ma.clone());

    for &atom in &ma.atoms {
        map.entry(atom as i32).or_default().push(ma.clone());
    }
}

pub fn collect_mana_payment_sources(
    game: &GameState,
    player: PlayerId,
    reserved_sacrifices: &[CardId],
) -> ManaPaymentSources {
    let source_cards = get_available_mana_sources(game, player, reserved_sacrifices);
    let mut mana_ability_options = Vec::new();

    for &card_id in &source_cards {
        let card = game.card(card_id);
        for ab in &card.activated_abilities {
            if !is_payable_mana_ability(game, player, card_id, ab, reserved_sacrifices, None) {
                continue;
            }
            mana_ability_options.push(ManaAbilityOption {
                card_id,
                ability_index: ab.ability_index,
                description: ab.ability_text.clone(),
            });
        }
    }

    ManaPaymentSources {
        source_cards,
        mana_ability_options,
    }
}

/// Mirrors Java harness `ActionSpace.canPayManaCostWithReservedSacrifices(...)`
/// for activated-ability action-space checks that reserve self/original-host
/// sacrifices while probing mana feasibility.
pub fn can_pay_mana_cost_with_reserved_sacrifices(
    game: &GameState,
    pool: &ManaPool,
    player: PlayerId,
    excluded_source: CardId,
    cost: &crate::cost::Cost,
    reserved_sacrifices: &[CardId],
    payment_ctx: Option<&crate::mana::ManaPaymentContext>,
) -> bool {
    let mana_cost = mana_cost_from_cost(cost);
    let available = crate::mana::calculate_available_mana_with_context(
        pool,
        game,
        player,
        Some(excluded_source),
        reserved_sacrifices,
        payment_ctx,
    );

    if payment_ctx.is_some() {
        available.can_pay_for_spell(&mana_cost, payment_ctx.unwrap())
            || available.can_pay_with_phyrexian_life(&mana_cost, game.player(player).life)
    } else {
        available.can_pay(&mana_cost)
            || available.can_pay_with_phyrexian_life(&mana_cost, game.player(player).life)
    }
}

/// Java-style action-space mana feasibility for spells with phyrexian costs.
///
/// This mirrors the wiring of `ComputerUtilMana.canPayManaCost(...)` in test mode:
/// pay from floating mana first, greedily commit mana sources shard-by-shard,
/// and only finish with life if the remaining unpaid cost is purely phyrexian.
///
/// This helper is intentionally for action-space / playability probing. Real
/// payment continues to use the stricter payment path.
pub fn can_pay_spell_mana_cost_for_action_space(
    game: &GameState,
    pool: &ManaPool,
    player: PlayerId,
    current_spell: CardId,
    cost: &forge_foundation::ManaCost,
    payment_ctx: &crate::mana::ManaPaymentContext,
) -> bool {
    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, pool);
    if unpaid.is_paid() {
        return true;
    }

    let mut used_sources = std::collections::HashSet::new();
    let mut guard = 0u32;
    while !unpaid.is_paid() && guard < 128 {
        guard += 1;

        let mana_ability_map = group_sources_by_mana_color(game, player, &[], Some(payment_ctx));
        if mana_ability_map.is_empty() {
            break;
        }

        let mut candidates = collect_sorted_candidates(game, player, &mana_ability_map);
        candidates.retain(|candidate| {
            !used_sources.contains(&candidate.card_id) && candidate.card_id != current_spell
        });
        if candidates.is_empty() {
            break;
        }

        let Some((sa_payment, to_pay)) = choose_candidate(
            game,
            player,
            Some(current_spell),
            &candidates,
            &unpaid,
            false,
            &[],
        ) else {
            break;
        };

        let Some(chosen_atom) = choose_atom_for_shard(&sa_payment, to_pay) else {
            break;
        };

        if let Some(fixed_atoms) = fixed_produced_atoms(&sa_payment.mana_text, &[]) {
            let repeats = (sa_payment.amount.max(1) as usize)
                .checked_div(fixed_atoms.len().max(1))
                .unwrap_or(1)
                .max(1);
            for _ in 0..repeats {
                for &atom in &fixed_atoms {
                    let _ = unpaid.try_pay_mana(atom, atom as u8);
                }
            }
        } else {
            let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
            for _ in 1..sa_payment.amount.max(1) {
                let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
            }
        }

        used_sources.insert(sa_payment.card_id);
    }

    unpaid.is_paid()
        || (unpaid.contains_only_phyrexian_mana()
            && game.player(player).life > required_phyrexian_life(&unpaid))
}

fn get_available_mana_sources(
    game: &GameState,
    player: PlayerId,
    reserved_sacrifices: &[CardId],
) -> Vec<CardId> {
    let mut sources: Vec<CardId> = game
        .cards_in_zone(ZoneType::Battlefield, player)
        .iter()
        .copied()
        .collect();

    for &cid in game.cards_in_zone(ZoneType::Hand, player) {
        let card = game.card(cid);
        if card
            .activated_abilities
            .iter()
            .any(|ab| is_payable_mana_ability(game, player, cid, ab, reserved_sacrifices, None))
        {
            sources.push(cid);
        }
    }

    sources.retain(|&cid| {
        let card = game.card(cid);
        for ab in &card.activated_abilities {
            if is_payable_mana_ability(game, player, cid, ab, reserved_sacrifices, None) {
                return true;
            }
        }
        if card.zone != ZoneType::Battlefield || card.tapped || !card.is_land() {
            return false;
        }
        let has_subtype = !all_basic_subtype_atoms(card).is_empty();
        let has_basic = basic_land_mana_atom(card).is_some();
        has_subtype || has_basic
    });
    sources
}

fn is_payable_mana_ability(
    game: &GameState,
    player: PlayerId,
    card_id: CardId,
    ab: &crate::ability::activated::ActivatedAbility,
    reserved_sacrifices: &[CardId],
    payment_ctx: Option<&crate::mana::ManaPaymentContext>,
) -> bool {
    if !ab.is_mana_ability {
        return false;
    }
    let card = game.card(card_id);
    let activation_zone = ab.params.get(keys::ACTIVATION_ZONE);
    match card.zone {
        ZoneType::Battlefield => {
            if activation_zone == Some("Hand") {
                return false;
            }
        }
        ZoneType::Hand => {
            if activation_zone != Some("Hand") {
                return false;
            }
        }
        _ => return false,
    }
    if ab
        .cost
        .parts
        .iter()
        .any(|p| matches!(p, CostPart::Mana { .. }))
    {
        return false;
    }
    if !can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
        return false;
    }
    if let Some(ctx) = payment_ctx {
        if let Some(raw) = ab.params.get(keys::RESTRICT_VALID) {
            let card = game.card(card_id);
            let resolved = if raw.contains("ChosenType") {
                let chosen = card.chosen_type.clone().unwrap_or_default();
                raw.replace("ChosenType", &chosen)
            } else {
                raw.to_string()
            };
            if !crate::mana::mana_meets_restriction(&resolved, ctx) {
                return false;
            }
        }
    }
    can_pay_mana_ability_costs_with_reserved(
        game,
        player,
        card_id,
        &ab.cost.parts,
        reserved_sacrifices,
    )
}

fn can_pay_mana_ability_costs_with_reserved(
    game: &GameState,
    player: PlayerId,
    source_id: CardId,
    cost_parts: &[CostPart],
    reserved_sacrifices: &[CardId],
) -> bool {
    for part in cost_parts {
        if !can_pay_source_paid_mana_cost_part(
            game,
            player,
            source_id,
            part,
            None,
            true,
            reserved_sacrifices,
        ) {
            return false;
        }
    }
    true
}

fn required_phyrexian_life(unpaid: &ManaCostBeingPaid) -> i32 {
    unpaid
        .get_distinct_shards()
        .into_iter()
        .filter(|shard| shard.is_phyrexian())
        .map(|shard| unpaid.get_unpaid_shards(shard) * 2)
        .sum()
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn colors_most_common_in_hand(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
) -> Vec<u16> {
    let mut max_pips = [0_i32; 5];
    for &card_id in game.cards_in_zone(forge_foundation::ZoneType::Hand, player) {
        if Some(card_id) == current_spell {
            continue;
        }
        let card = game.card(card_id);
        if card.is_land() {
            continue;
        }

        let mut pips = [0_i32; 5];
        for shard in card.mana_cost.shards() {
            let atoms = shard.shard() & ManaAtom::COLORS_SUPERPOSITION;
            if (atoms & ManaAtom::WHITE) != 0 {
                pips[0] += 1;
            }
            if (atoms & ManaAtom::BLUE) != 0 {
                pips[1] += 1;
            }
            if (atoms & ManaAtom::BLACK) != 0 {
                pips[2] += 1;
            }
            if (atoms & ManaAtom::RED) != 0 {
                pips[3] += 1;
            }
            if (atoms & ManaAtom::GREEN) != 0 {
                pips[4] += 1;
            }
        }

        for i in 0..5 {
            max_pips[i] = max_pips[i].max(pips[i]);
        }
    }

    let mut ordered = vec![
        (ManaAtom::WHITE, max_pips[0]),
        (ManaAtom::BLUE, max_pips[1]),
        (ManaAtom::BLACK, max_pips[2]),
        (ManaAtom::RED, max_pips[3]),
        (ManaAtom::GREEN, max_pips[4]),
    ];

    ordered.sort_by(|a, b| b.1.cmp(&a.1));
    ordered
        .into_iter()
        .filter_map(|(atom, count)| if count > 0 { Some(atom) } else { None })
        .collect()
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn score_mana_producing_card(game: &GameState, card_id: CardId, player: PlayerId) -> i32 {
    let card = game.card(card_id);
    let mut score = 0;
    let mut has_mana_ability = false;

    for ab in &card.activated_abilities {
        if ab.is_mana_ability {
            let produced = ab.params.get(keys::PRODUCED);
            score += score_mana_ability(game, card_id, ab, produced);
            has_mana_ability = true;
        } else if can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
            score += 13;
        }
    }

    if !has_mana_ability && card.is_land() {
        let mut subtype_atoms = all_basic_subtype_atoms(card);
        if subtype_atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(card) {
                subtype_atoms.push(a);
            }
        }
        for atom in subtype_atoms {
            score += score_implicit_land_mana_ability(atom);
        }
    }

    if card.can_attack() {
        score += 13;
    }
    if card.can_block() {
        score += 13;
    }

    score
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn score_mana_ability(
    game: &GameState,
    card_id: CardId,
    ab: &crate::ability::activated::ActivatedAbility,
    produced_override: Option<&str>,
) -> i32 {
    let mut score = 0;
    let card = game.card(card_id);

    if let Some(produced) = produced_override.or_else(|| ab.params.get(keys::PRODUCED)) {
        let mana_text = ability_mana_text_for_score(produced, &card.chosen_colors);
        if mana_text == "Any" {
            score += 7;
        } else {
            score += mana_text.len() as i32;
            if !mana_text.contains('C') {
                score += 1;
            }
        }
    } else {
        score += 1;
    }

    for part in &ab.cost.parts {
        match part {
            CostPart::PayLife(_) => score += 3,
            CostPart::Sacrifice { type_filter, .. } => {
                score += 6;
                if type_filter != "CARDNAME" {
                    score += 40;
                }
            }
            CostPart::Discard { .. } => score += 6,
            _ => {}
        }
        score += 1;
    }

    // Java parity: SpellAbility.calculateScoreForManaAbility() adds +50 for
    // non-undoable abilities (those with side-effect SubAbilities like DealDamage).
    // Also adds +2 for any SubAbility presence. This heavily de-prioritizes pain
    // lands (e.g. Yavimaya Coast's colored mana ability with DealDamage sub).
    if let Some(sub_name) = ab.params.get(keys::SUB_ABILITY) {
        score += 2;
        // Check if the SubAbility is non-undoable (damage, discard, etc.)
        if let Some(sub_text) = card.svars.get(sub_name) {
            let sub_params = crate::parsing::Params::from_raw(sub_text);
            let sub_type = sub_params.get(crate::parsing::keys::DB).unwrap_or("");
            if matches!(
                sub_type,
                "DealDamage" | "LoseLife" | "Discard" | "Destroy" | "Sacrifice" | "Mill"
            ) {
                score += 50; // non-undoable: only use as last resort
            }
        }
    }

    score
}

/// Sort per-shard source lists to match Java AutoPay's ManaAbilityCandidate.score().
/// Lower scores are picked first. Lands score low; creatures score high (+26).
/// This ensures lands are tapped before valuable mana dorks.
fn sort_sources_for_autopay(
    game: &GameState,
    player: PlayerId,
    sources_for_shards: &mut IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
) {
    for abilities in sources_for_shards.values_mut() {
        abilities.sort_by(|a, b| {
            // Score per-ability (not per-card) so that different abilities on the same
            // card (e.g. Yavimaya Coast's {C} vs {G}/{U}) get accurate individual scores.
            let sa = autopay_source_score(game, player, a) * 1000 + a.source_order as i32;
            let sb = autopay_source_score(game, player, b) * 1000 + b.source_order as i32;
            sa.cmp(&sb)
        });
    }
}

/// Score a mana source for AutoPay sorting, mirroring Java AutoPay's
/// ManaAbilityCandidate.score():
/// - Mana ability score based on produced colors
/// - +cost_parts.size() for activation cost complexity
/// - +13 per combat role (attack/block) for creatures
fn autopay_source_score(game: &GameState, _player: PlayerId, ma: &ManaAbilityRef) -> i32 {
    let card = game.card(ma.card_id);
    if let Some(ab_idx) = ma.ability_index {
        if let Some(ab) = card.activated_abilities.get(ab_idx) {
            let resolved = if ma.atoms.len() >= 5
                && !ma.atoms.contains(&ManaAtom::COLORLESS)
                && !ma.atoms.contains(&ManaAtom::GENERIC)
            {
                "Any".to_string()
            } else {
                ma.atoms
                    .iter()
                    .copied()
                    .filter(|&atom| atom != ManaAtom::GENERIC)
                    .map(atom_short)
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let mut s = score_mana_ability(game, ma.card_id, ab, Some(&resolved));
            if card.is_creature() {
                s += 13;
                s += 13;
            }
            return s;
        }
    }

    let mut s =
        score_implicit_land_mana_ability(ma.atoms.first().copied().unwrap_or(ManaAtom::COLORLESS));
    if card.is_creature() {
        s += 13;
        s += 13;
    }

    s
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn score_implicit_land_mana_ability(atom: u16) -> i32 {
    let mut score = 0;
    let text = atom_short(atom);
    score += text.len() as i32;
    if atom != ManaAtom::COLORLESS {
        score += 1;
    }
    score += 1;
    score
}

fn ability_mana_text_for_score(produced: &str, chosen_colors: &[String]) -> String {
    if produced.eq_ignore_ascii_case("Any") {
        return "Any".to_string();
    }
    let atoms = produced_to_atoms(produced, chosen_colors);
    if atoms.is_empty() {
        return String::new();
    }

    atoms
        .into_iter()
        .map(atom_short)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Auto-tap untapped lands to produce `needed` additional generic mana.
/// Used for paying commander tax on top of the regular cost.
pub fn auto_tap_lands_generic(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    needed: i32,
) -> Vec<CardId> {
    let deficit = (needed - pool.total_mana()).max(0);
    if deficit <= 0 {
        return Vec::new();
    }

    let mut remaining = deficit;
    let mut tapped_lands: Vec<CardId> = Vec::new();

    for card_id in get_available_mana_sources(game, player, &[]) {
        if remaining <= 0 {
            break;
        }
        let card = game.card(card_id);
        if !card.is_land() || card.tapped {
            continue;
        }
        let mut atoms = all_basic_subtype_atoms(card);
        if atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(card) {
                atoms.push(a);
            }
        }

        let atom = if atoms.contains(&ManaAtom::COLORLESS) {
            ManaAtom::COLORLESS
        } else {
            atoms.first().copied().unwrap_or(ManaAtom::COLORLESS)
        };

        tap_land_for_mana(game, pool, player, card_id, atom, true, &mut tapped_lands);
        remaining -= 1;
    }

    tapped_lands
}

fn source_requires_tap(game: &GameState, ma: &ManaAbilityRef) -> bool {
    match ma.ability_index {
        // Implicit mana abilities (basic/subtype lands) always require tapping.
        None => true,
        Some(ab_idx) => game.card(ma.card_id).activated_abilities[ab_idx]
            .cost
            .parts
            .iter()
            .any(|p| matches!(p, CostPart::Tap)),
    }
}

/// Forward-ported from Java for future use. Currently unused but will be needed
/// when mana payment prioritization logic is fully ported.
#[allow(dead_code)]
fn parse_mana_ability_amount(ab: &crate::ability::activated::ActivatedAbility) -> i32 {
    parse_mana_ability_amount_with_game(ab, None, None, None)
}

/// Resolve the Amount param for a mana ability, supporting SVar expressions
/// like `IncubationAmount` → `Count$Compare Y GE1.3.1`.
fn parse_mana_ability_amount_with_game(
    ab: &crate::ability::activated::ActivatedAbility,
    game: Option<&GameState>,
    card_id: Option<CardId>,
    player: Option<PlayerId>,
) -> i32 {
    let Some(amount_str) = ab.params.get(keys::AMOUNT) else {
        return 1;
    };
    // Try direct integer parse first
    if let Ok(n) = amount_str.parse::<i32>() {
        return if n > 0 { n } else { 1 };
    }
    // It's an SVar reference — resolve it using the source card's SVars
    if let (Some(game), Some(cid), Some(pid)) = (game, card_id, player) {
        if let Some(svar_expr) = game.card(cid).svars.get(amount_str) {
            if svar_expr.starts_with("Count$") {
                return crate::ability::effects::resolve_count_svar(svar_expr, game, cid, pid);
            }
            return svar_expr.parse::<i32>().unwrap_or(1);
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use forge_foundation::{CardTypeLine, ColorSet};

    fn make_card(
        id: u32,
        owner: PlayerId,
        name: &str,
        type_line: &str,
        abilities: Vec<&str>,
    ) -> Card {
        Card::new(
            CardId(id),
            name.to_string(),
            owner,
            CardTypeLine::parse(type_line),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            abilities.into_iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn auto_tap_does_not_spend_reserved_source_on_mana_sacrifice_costs_by_default() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let player = PlayerId(0);
        let mut pool = ManaPool::new();

        let reserved_food = game.create_card(make_card(
            1,
            player,
            "Food Token",
            "Artifact Food",
            vec!["AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3"],
        ));
        let goose = game.create_card(make_card(
            2,
            player,
            "Gilded Goose",
            "Creature Bird",
            vec!["AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any"],
        ));
        let forest = game.create_card(make_card(
            3,
            player,
            "Forest",
            "Land Forest",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        game.zone_mut(ZoneType::Battlefield, player)
            .add(reserved_food);
        game.zone_mut(ZoneType::Battlefield, player).add(goose);
        game.zone_mut(ZoneType::Battlefield, player).add(forest);
        game.card_mut(reserved_food).zone = ZoneType::Battlefield;
        game.card_mut(goose).zone = ZoneType::Battlefield;
        game.card_mut(forest).zone = ZoneType::Battlefield;
        game.card_mut(reserved_food).summoning_sick = false;
        game.card_mut(goose).summoning_sick = false;
        game.card_mut(forest).summoning_sick = false;

        let tapped = auto_tap_lands(
            &mut game,
            &mut pool,
            player,
            &ManaCost::parse("2"),
            Some(reserved_food),
        );

        assert_eq!(pool.total_mana(), 1);
        assert_eq!(tapped, vec![forest]);
        assert!(!game.card(goose).tapped);
        assert_eq!(game.card(goose).zone, ZoneType::Battlefield);
        assert_eq!(game.card(reserved_food).zone, ZoneType::Battlefield);
    }

    #[test]
    fn auto_tap_can_spend_reserved_source_when_explicitly_allowed() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let player = PlayerId(0);
        let mut pool = ManaPool::new();

        let reserved_food = game.create_card(make_card(
            1,
            player,
            "Food Token",
            "Artifact Food",
            vec!["AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3"],
        ));
        let goose = game.create_card(make_card(
            2,
            player,
            "Gilded Goose",
            "Creature Bird",
            vec!["AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any"],
        ));
        let forest = game.create_card(make_card(
            3,
            player,
            "Forest",
            "Land Forest",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        for cid in [reserved_food, goose, forest] {
            game.zone_mut(ZoneType::Battlefield, player).add(cid);
            game.card_mut(cid).zone = ZoneType::Battlefield;
            game.card_mut(cid).summoning_sick = false;
        }

        let tapped = auto_tap_lands_allow_reserved_source_reuse(
            &mut game,
            &mut pool,
            player,
            &ManaCost::parse("2"),
            Some(reserved_food),
        );

        assert_eq!(pool.total_mana(), 2);
        // Auto-tapper prefers simpler sources: Forest (score 3) before Goose (score 35).
        assert_eq!(tapped, vec![forest, goose]);
        assert!(game.card(goose).tapped);
        assert!(game.card(forest).tapped);
        assert_eq!(game.card(goose).zone, ZoneType::Battlefield);
        assert_eq!(game.card(reserved_food).zone, ZoneType::Graveyard);
    }

    #[test]
    fn auto_tap_uses_battlefield_order_for_generic_payment() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let player = PlayerId(0);
        let mut pool = ManaPool::new();

        let plains = game.create_card(make_card(
            1,
            player,
            "Plains",
            "Land",
            vec!["AB$ Mana | Cost$ T | Produced$ W"],
        ));
        let mountain = game.create_card(make_card(
            2,
            player,
            "Mountain",
            "Land",
            vec!["AB$ Mana | Cost$ T | Produced$ R"],
        ));
        let forest = game.create_card(make_card(
            3,
            player,
            "Forest",
            "Land Forest",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        for cid in [plains, mountain, forest] {
            game.zone_mut(ZoneType::Battlefield, player).add(cid);
            game.card_mut(cid).zone = ZoneType::Battlefield;
            game.card_mut(cid).summoning_sick = false;
        }

        let tapped = auto_tap_lands(&mut game, &mut pool, player, &ManaCost::parse("2"), None);

        assert_eq!(pool.total_mana(), 2);
        assert_eq!(tapped, vec![plains, mountain]);
        assert!(game.card(plains).tapped);
        assert!(game.card(mountain).tapped);
        assert!(!game.card(forest).tapped);
    }

    #[test]
    fn auto_tap_calls_confirm_payment_for_self_sacrifice() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let player = PlayerId(0);

        // Create a Treasure Token (self-sacrifice for mana)
        let treasure = game.create_card(make_card(
            1,
            player,
            "Treasure Token",
            "Artifact Treasure",
            vec!["AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any"],
        ));

        game.zone_mut(ZoneType::Battlefield, player).add(treasure);
        game.card_mut(treasure).zone = ZoneType::Battlefield;
        game.card_mut(treasure).summoning_sick = false;

        // Test 1: confirm_payment returns true (ACCEPT)
        {
            let mut pool = ManaPool::new();
            let tapped = {
                let game_ptr: *mut GameState = &mut game;
                let mut callback = |kind: ManaPayCallback<'_>| -> Option<CardId> {
                    match kind {
                        ManaPayCallback::ChooseSacrifice(_) => None,
                        ManaPayCallback::ChooseColor(_) => None,
                        ManaPayCallback::ConfirmSelfSacrifice(cid) => {
                            assert_eq!(cid, treasure); // should be asking about Treasure
                            Some(cid) // confirm
                        }
                        ManaPayCallback::ConfirmSubCounter(cid) => Some(cid),
                        ManaPayCallback::ConfirmSourceExile(cid) => Some(cid),
                        ManaPayCallback::NotifySacrificeForMana(cid) => unsafe {
                            let game = &mut *game_ptr;
                            let owner = game.card(cid).owner;
                            game.move_card(cid, ZoneType::Graveyard, owner);
                            Some(cid)
                        },
                    }
                };

                auto_tap_lands_with_callbacks(
                    &mut game,
                    &mut pool,
                    player,
                    &ManaCost::parse("1"),
                    None,
                    &mut callback,
                )
            };

            // The confirm callback was called if the treasure was sacrificed
            assert_eq!(tapped, vec![treasure]);
            assert_eq!(game.card(treasure).zone, ZoneType::Graveyard);
            assert_eq!(pool.total_mana(), 1);
        }

        // Reset for test 2: create new treasure and add a Forest as fallback
        let treasure2 = game.create_card(make_card(
            2,
            player,
            "Treasure Token",
            "Artifact Treasure",
            vec!["AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any"],
        ));
        let forest = game.create_card(make_card(
            3,
            player,
            "Forest",
            "Land Forest",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));
        game.zone_mut(ZoneType::Battlefield, player).add(treasure2);
        game.zone_mut(ZoneType::Battlefield, player).add(forest);
        game.card_mut(treasure2).zone = ZoneType::Battlefield;
        game.card_mut(treasure2).summoning_sick = false;
        game.card_mut(forest).zone = ZoneType::Battlefield;
        game.card_mut(forest).summoning_sick = false;

        // Test 2: confirm_payment returns false (DECLINE)
        {
            let mut pool = ManaPool::new();
            let tapped = {
                let game_ptr: *mut GameState = &mut game;
                let mut callback = |kind: ManaPayCallback<'_>| -> Option<CardId> {
                    match kind {
                        ManaPayCallback::ChooseSacrifice(_) => None,
                        ManaPayCallback::ChooseColor(_) => None,
                        ManaPayCallback::ConfirmSelfSacrifice(cid) => {
                            assert_eq!(cid, treasure2);
                            None // decline
                        }
                        ManaPayCallback::ConfirmSubCounter(cid) => Some(cid),
                        ManaPayCallback::ConfirmSourceExile(cid) => Some(cid),
                        ManaPayCallback::NotifySacrificeForMana(cid) => unsafe {
                            let game = &mut *game_ptr;
                            let owner = game.card(cid).owner;
                            game.move_card(cid, ZoneType::Graveyard, owner);
                            Some(cid)
                        },
                    }
                };

                auto_tap_lands_with_callbacks(
                    &mut game,
                    &mut pool,
                    player,
                    &ManaCost::parse("1"),
                    None,
                    &mut callback,
                )
            };

            // When declined, should fall back to Forest
            assert_eq!(tapped, vec![forest]);
            assert_eq!(game.card(treasure2).zone, ZoneType::Battlefield); // not sacrificed
            assert_eq!(game.card(forest).zone, ZoneType::Battlefield);
            assert_eq!(pool.total_mana(), 1);
        }
    }
}
