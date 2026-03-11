use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ManaCostShard, ZoneType};
use indexmap::IndexMap;
use std::collections::HashMap;

use crate::cost::{can_pay_ignoring_mana, get_sacrifice_targets, CostPart};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use super::mana_cost_being_paid::{can_pay_for_shard_with_color, ManaCostBeingPaid};
use super::{
    all_basic_subtype_atoms, atom_short, basic_land_mana_atom, compute_reflected_atoms,
    produced_to_atoms, tap_land_for_mana, ManaPool,
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
    auto_tap_lands_internal(game, pool, player, cost, current_spell, false)
}

pub fn auto_tap_lands_allow_reserved_source_reuse(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<CardId> {
    auto_tap_lands_internal(game, pool, player, cost, current_spell, true)
}

fn auto_tap_lands_internal(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
    allow_reserved_source_reuse: bool,
) -> Vec<CardId> {
    let mut tapped_lands: Vec<CardId> = Vec::new();

    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, pool);
    if unpaid.is_paid() {
        return tapped_lands;
    }

    let mana_ability_map = group_sources_by_mana_color(game, player);
    if mana_ability_map.is_empty() {
        return tapped_lands;
    }

    let mut sources_for_shards = group_and_order_to_pay_shards(&mana_ability_map, &unpaid);
    if sources_for_shards.is_empty() {
        return tapped_lands;
    }

    // Sort per-shard source lists so lands are tapped before creatures.
    // Mirrors Java AutoPay's ManaAbilityCandidate.score() sort which assigns
    // lower scores to lands and higher scores (+26) to creatures.
    sort_sources_for_autopay(game, player, &mut sources_for_shards);

    while !unpaid.is_paid() {
        let Some(to_pay) = get_next_shard_to_pay(&unpaid, &sources_for_shards) else {
            break;
        };

        let ma_list = sources_for_shards.get(&to_pay).cloned().unwrap_or_default();
        if ma_list.is_empty() {
            break;
        }

        let Some(sa_payment) = choose_mana_ability(
            game,
            player,
            current_spell,
            to_pay,
            &ma_list,
            allow_reserved_source_reuse,
        )
        else {
            break;
        };

        let Some(chosen_atom) = choose_atom_for_shard(&sa_payment, to_pay)
        else {
            break;
        };

        // Pay non-tap ability costs first so a failed payment cannot generate mana.
        if !pay_non_tap_mana_ability_costs(
            game,
            player,
            &sa_payment,
            current_spell,
            allow_reserved_source_reuse,
        ) {
            break;
        }

        tap_land_for_mana(
            game,
            pool,
            player,
            sa_payment.card_id,
            chosen_atom,
            &mut tapped_lands,
        );

        let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
        for _ in 1..sa_payment.amount.max(1) {
            pool.add(chosen_atom, 1);
            let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
        }

        // Sources can only be used once; remove all entries from same host card.
        for abilities in sources_for_shards.values_mut() {
            abilities.retain(|a| a.card_id != sa_payment.card_id);
        }
    }

    tapped_lands
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

fn choose_mana_ability(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    to_pay: ManaCostShard,
    ma_list: &[ManaAbilityRef],
    allow_reserved_source_reuse: bool,
) -> Option<ManaAbilityRef> {
    for ma in ma_list {
        // Java ComputerUtilMana.chooseManaAbility() skips mana abilities on the
        // same host card as the spell/ability currently being paid for.
        if Some(ma.card_id) == current_spell {
            continue;
        }
        if ma.can_pay_shard(to_pay)
            && can_pay_non_tap_mana_ability_costs(
                game,
                player,
                ma,
                current_spell,
                allow_reserved_source_reuse,
            )
        {
            return Some(ma.clone());
        }
    }
    None
}

fn can_pay_non_tap_mana_ability_costs(
    game: &GameState,
    player: PlayerId,
    ma: &ManaAbilityRef,
    reserved_source: Option<CardId>,
    allow_reserved_source_reuse: bool,
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
            CostPart::Tap | CostPart::Mana(_) => {}
            CostPart::PayLife(amount) => {
                if game.player(player).life < *amount {
                    return false;
                }
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                if game.card(ma.card_id).counter_count(counter_type) < *amount {
                    return false;
                }
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if *amount > 1 || game.card(ma.card_id).zone != ZoneType::Battlefield {
                        return false;
                    }
                } else {
                    let mut targets = get_sacrifice_targets(game, player, type_filter);
                    if !allow_reserved_source_reuse {
                        if let Some(reserved) = reserved_source {
                            targets.retain(|&cid| cid != reserved);
                        }
                    }
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            _ => return false,
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
            CostPart::Tap | CostPart::Mana(_) => {}
            CostPart::PayLife(amount) => {
                if game.player(player).life < *amount {
                    return false;
                }
                game.player_mut(player).lose_life(*amount);
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                if game.card(ma.card_id).counter_count(counter_type) < *amount {
                    return false;
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
                    let owner = game.card(ma.card_id).owner;
                    game.move_card(ma.card_id, ZoneType::Graveyard, owner);
                } else {
                    let mut targets = get_sacrifice_targets(game, player, type_filter);
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
                    for cid in targets.into_iter().take(required) {
                        let owner = game.card(cid).owner;
                        game.move_card(cid, ZoneType::Graveyard, owner);
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

fn choose_atom_for_shard(
    mana_ab: &ManaAbilityRef,
    shard: ManaCostShard,
) -> Option<u16> {
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
) -> IndexMap<i32, Vec<ManaAbilityRef>> {
    let mut mana_map: IndexMap<i32, Vec<ManaAbilityRef>> = IndexMap::new();
    let mut source_order = 0usize;

    for card_id in get_available_mana_sources(game, player) {
        let card = game.card(card_id);
        let mut explicit_mana_added = false;

        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            if ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_))) {
                continue;
            }
            if !can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
                continue;
            }
            // Handle ManaReflected abilities (e.g. Incubation Druid)
            if ab.params.get("AB").map_or(false, |v| v == "ManaReflected") {
                let reflected_atoms = compute_reflected_atoms(game, player, card_id, ab);
                if !reflected_atoms.is_empty() {
                    explicit_mana_added = true;
                    let ma = ManaAbilityRef {
                        card_id,
                        ability_index: Some(ab.ability_index),
                        atoms: reflected_atoms,
                        amount: parse_mana_ability_amount_with_game(ab, Some(game), Some(card_id), Some(player)),
                        mana_text: "Reflected".to_string(),
                        source_order,
                    };
                    source_order += 1;
                    add_mana_ability_to_color_map(&mut mana_map, &ma);
                }
                continue;
            }

            let Some(produced) = ab.params.get("Produced") else {
                continue;
            };
            if produced == "Combo ColorIdentity" {
                continue;
            }

            let atoms = produced_to_atoms(produced, &card.chosen_colors);
            if atoms.is_empty() {
                continue;
            }

            explicit_mana_added = true;
            let ma = ManaAbilityRef {
                card_id,
                ability_index: Some(ab.ability_index),
                atoms: atoms.clone(),
                amount: parse_mana_ability_amount_with_game(ab, Some(game), Some(card_id), Some(player)),
                mana_text: ability_mana_text_for_score(produced, &card.chosen_colors),
                source_order,
            };
            source_order += 1;
            add_mana_ability_to_color_map(&mut mana_map, &ma);
        }

        if !explicit_mana_added {
            let mut atoms = all_basic_subtype_atoms(card);
            if atoms.is_empty() {
                if let Some(a) = basic_land_mana_atom(card) {
                    atoms.push(a);
                }
            }
            for atom in atoms {
                let ma = ManaAbilityRef {
                    card_id,
                    ability_index: None,
                    atoms: vec![atom],
                    amount: 1,
                    mana_text: atom_short(atom).to_string(),
                    source_order,
                };
                source_order += 1;
                add_mana_ability_to_color_map(&mut mana_map, &ma);
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

fn get_available_mana_sources(game: &GameState, player: PlayerId) -> Vec<CardId> {
    let mut sources: Vec<CardId> = game
        .cards_in_zone(forge_foundation::ZoneType::Battlefield, player)
        .iter()
        .copied()
        .collect();
    sources.retain(|&cid| {
        let card = game.card(cid);
        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            if ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_))) {
                continue;
            }
            if can_pay_ignoring_mana(&ab.cost, game, cid, player) {
                return true;
            }
        }
        if card.tapped || !card.is_land() {
            return false;
        }
        let has_subtype = !all_basic_subtype_atoms(card).is_empty();
        let has_basic = basic_land_mana_atom(card).is_some();
        has_subtype || has_basic
    });
    sources
}

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

fn score_mana_producing_card(game: &GameState, card_id: CardId, player: PlayerId) -> i32 {
    let card = game.card(card_id);
    let mut score = 0;
    let mut has_mana_ability = false;

    for ab in &card.activated_abilities {
        if ab.is_mana_ability {
            let produced = ab.params.get("Produced").map(|s| s.as_str());
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

fn score_mana_ability(
    game: &GameState,
    card_id: CardId,
    ab: &crate::ability::activated::ActivatedAbility,
    produced_override: Option<&str>,
) -> i32 {
    let mut score = 0;
    let card = game.card(card_id);

    if let Some(produced) =
        produced_override.or_else(|| ab.params.get("Produced").map(String::as_str))
    {
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
    if let Some(sub_name) = ab.params.get("SubAbility") {
        score += 2;
        // Check if the SubAbility is non-undoable (damage, discard, etc.)
        if let Some(sub_text) = card.svars.get(sub_name) {
            let sub_params = crate::trigger::parse_pipe_params(sub_text);
            let sub_type = sub_params.get("DB").map(String::as_str).unwrap_or("");
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
    let mut s: i32 = 0;

    // Mana ability intrinsic score.
    if ma.mana_text == "Any" || ma.mana_text == "Reflected" {
        // Any-mana and reflected abilities are flexible → higher score.
        s += 7;
    } else {
        let words: Vec<&str> = ma.mana_text.split_whitespace().collect();
        s += words.len() as i32;
        if !ma.mana_text.contains('C') {
            s += 1;
        }
    }

    // Cost complexity.
    if let Some(ab_idx) = ma.ability_index {
        if let Some(ab) = card.activated_abilities.get(ab_idx) {
            s += ab.cost.parts.len() as i32;
        }
    } else {
        // Implicit land tap: 1 cost part (tap).
        s += 1;
    }

    // Creatures with combat potential are more valuable.
    if card.is_creature() {
        if card.can_attack() {
            s += 13;
        }
        if card.can_block() {
            s += 13;
        }
    }

    s
}

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
    let deficit = (needed - pool.total()).max(0);
    if deficit <= 0 {
        return Vec::new();
    }

    let mut remaining = deficit;
    let mut tapped_lands: Vec<CardId> = Vec::new();

    for card_id in get_available_mana_sources(game, player) {
        if remaining <= 0 {
            break;
        }
        let card = game.card(card_id);
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

        tap_land_for_mana(game, pool, player, card_id, atom, &mut tapped_lands);
        remaining -= 1;
    }

    tapped_lands
}
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
    let Some(amount_str) = ab.params.get("Amount") else {
        return 1;
    };
    // Try direct integer parse first
    if let Ok(n) = amount_str.parse::<i32>() {
        return if n > 0 { n } else { 1 };
    }
    // It's an SVar reference — resolve it using the source card's SVars
    if let (Some(game), Some(cid), Some(pid)) = (game, card_id, player) {
        if let Some(svar_expr) = game.card(cid).svars.get(amount_str.as_str()) {
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
    use crate::card::CardInstance;
    use forge_foundation::{CardTypeLine, ColorSet};

    fn make_card(
        id: u32,
        owner: PlayerId,
        name: &str,
        type_line: &str,
        abilities: Vec<&str>,
    ) -> CardInstance {
        CardInstance::new(
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
            "Land",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        game.zone_mut(ZoneType::Battlefield, player).add(reserved_food);
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

        assert_eq!(pool.total(), 1);
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
            "Land",
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

        assert_eq!(pool.total(), 2);
        assert_eq!(tapped, vec![goose, forest]);
        assert!(game.card(goose).tapped);
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
            "Land",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        for cid in [plains, mountain, forest] {
            game.zone_mut(ZoneType::Battlefield, player).add(cid);
            game.card_mut(cid).zone = ZoneType::Battlefield;
            game.card_mut(cid).summoning_sick = false;
        }

        let tapped = auto_tap_lands(
            &mut game,
            &mut pool,
            player,
            &ManaCost::parse("2"),
            None,
        );

        assert_eq!(pool.total(), 2);
        assert_eq!(tapped, vec![plains, mountain]);
        assert!(game.card(plains).tapped);
        assert!(game.card(mountain).tapped);
        assert!(!game.card(forest).tapped);
    }
}
