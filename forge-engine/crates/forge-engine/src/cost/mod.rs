mod cost_parser;
pub mod cost_add_mana;
pub mod cost_adjustment;
pub mod cost_behold;
pub mod cost_blight;
pub mod cost_choose_color;
pub mod cost_choose_creature_type;
pub mod cost_collect_evidence;
pub mod cost_damage;
pub mod cost_discard;
pub mod cost_draw;
pub mod cost_enlist;
pub mod cost_exert;
pub mod cost_exile;
pub mod cost_exile_ctrl_or_grave;
pub mod cost_exile_from_stack;
pub mod cost_exiled_move_to_grave;
pub mod cost_flip_coin;
pub mod cost_forage;
pub mod cost_gain_control;
pub mod cost_gain_life;
pub mod cost_mill;
pub mod cost_pay_energy;
pub mod cost_pay_life;
pub mod cost_pay_shards;
pub mod cost_payment;
pub mod cost_promise_gift;
pub mod cost_put_card_to_lib;
pub mod cost_put_counter;
pub mod cost_remove_any_counter;
pub mod cost_return;
pub mod cost_reveal;
pub mod cost_reveal_chosen;
pub mod cost_roll_dice;
pub mod cost_sacrifice;
pub mod cost_sub_counter;
pub mod cost_tap;
pub mod cost_tap_type;
pub mod cost_unattach;
pub mod cost_untap;
pub mod cost_untap_type;
pub mod cost_waterbend;
pub mod payment_decision;

use forge_foundation::{ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ability::effects::{matches_change_type, matches_valid_cards};
use crate::card::{Card, CounterType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::spellability::SpellAbility;
use crate::staticability::static_ability_cant_exile::cant_exile;
use crate::staticability::static_ability_cant_gain_lose_pay_life::cant_pay_life;
use crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card;
use crate::staticability::static_ability_cant_sacrifice::cant_sacrifice;

const DYNAMIC_X_SENTINEL: i32 = i32::MIN;

pub(super) fn parse_i32_or_x(inner: &str, default: i32) -> i32 {
    let trimmed = inner.trim();
    if trimmed.eq_ignore_ascii_case("X") {
        DYNAMIC_X_SENTINEL
    } else {
        trimmed.parse::<i32>().unwrap_or(default)
    }
}

pub fn resolve_dynamic_amount(
    game: &GameState,
    source: CardId,
    player: PlayerId,
    amount: i32,
) -> i32 {
    if amount != DYNAMIC_X_SENTINEL {
        return amount;
    }
    let source_card = game.card(source);

    if let Some(paid_x) = source_card
        .svars
        .get("XPaid")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return paid_x;
    }

    if let Some(x_expr) = source_card.svars.get("X") {
        if x_expr == "Count$xPaid" || x_expr == "Count$XPaid" {
            return source_card
                .svars
                .get("XPaid")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
        }
        if let Ok(n) = x_expr.parse::<i32>() {
            return n;
        }
        if x_expr.starts_with("Count$") {
            return crate::ability::effects::resolve_count_svar(x_expr, game, source, player);
        }
    }

    0
}

/// A single component of an ability cost.
/// Mirrors Java's CostPart hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevealFrom {
    Hand,
    Exile,
    HandOrBattlefield,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostPart {
    /// Tap the source permanent. {T}
    Tap,
    /// Pay mana.
    Mana(ManaCost),
    /// Pay life. Mirrors CostPayLife.
    PayLife(i32),
    /// Sacrifice permanents. type_filter "CARDNAME" means sacrifice self.
    Sacrifice { amount: i32, type_filter: String },
    /// Discard cards. type_filter "CARDNAME" means discard self.
    Discard { amount: i32, type_filter: String },
    /// Remove counters from the source permanent (e.g. SubCounter<1/DREAM/NICKNAME>).
    SubCounter {
        amount: i32,
        counter_type: CounterType,
    },
    /// Add counters to the source permanent (e.g. AddCounter<1/LOYALTY>). Mirrors CostPutCounter.
    AddCounter {
        amount: i32,
        counter_type: CounterType,
    },
    /// Exile cards from a specific zone (own zone) as cost. Mirrors CostExile.
    Exile {
        amount: i32,
        type_filter: String,
        from: ZoneType,
    },
    /// Exile cards from any player's graveyard as cost (ExileAnyGrave). Mirrors CostExile zoneMode=-1.
    ExileFromAnyGrave { amount: i32, type_filter: String },
    /// Exile cards from the same graveyard as cost (ExileSameGrave). Mirrors CostExile zoneMode=0.
    ExileFromSameGrave { amount: i32, type_filter: String },
    /// Return permanents to owner's hand as cost. Mirrors CostReturn.
    Return { amount: i32, type_filter: String },
    /// Tap other permanents of a type as cost (tapXType<n/filter>). Mirrors CostTapType.
    /// When `min_total_power` is Some(N), tap any number of creatures whose total power >= N
    /// (used by Crew). When None, tap exactly `amount` matching permanents.
    TapType {
        amount: i32,
        type_filter: String,
        min_total_power: Option<i32>,
    },
    /// Untap permanents as cost. Mirrors CostUntap.
    Untap,
    /// Untap other permanents of a type as cost (untapYType<n/filter>). Mirrors CostUntapType.
    UntapType { amount: i32, type_filter: String },
    /// Pay energy counters. Mirrors CostPayEnergy.
    PayEnergy(i32),
    /// Pay shard counters. Mirrors CostPayShards.
    PayShards(i32),
    /// Deal damage to the source's controller as cost. Mirrors CostDamage.
    DamageYou(i32),
    /// Draw cards as cost. Mirrors CostDraw.
    Draw(i32),
    /// Mill cards as cost. Mirrors CostMill.
    Mill(i32),
    /// Reveal cards as cost. Mirrors CostReveal.
    Reveal {
        amount: i32,
        type_filter: String,
        from: RevealFrom,
    },
    /// Exert permanent(s) as cost. Mirrors CostExert.
    Exert { amount: i32, type_filter: String },
    /// Opponent gains life as cost. Mirrors CostGainLife.
    GainLife(i32),
    /// Gain control of permanents matching type_filter as cost. Mirrors CostGainControl.
    GainControl { amount: i32, type_filter: String },
    /// Remove any counter type from permanents matching type_filter. Mirrors CostRemoveAnyCounter.
    /// `counter_type` is None means any counter type.
    RemoveAnyCounter {
        amount: i32,
        type_filter: String,
        counter_type: Option<CounterType>,
    },
    /// Unattach the source equipment from whatever it is equipping. Mirrors CostUnattach.
    Unattach,
    /// Move cards from exile to graveyard as cost. Mirrors CostExiledMoveToGrave.
    ExiledMoveToGrave { amount: i32, type_filter: String },
    /// Add mana to the pool as a cost (AddMana<amount/type>). Mirrors CostAddMana.
    /// Always payable. Payment adds the specified mana to the activating player's pool.
    AddMana { amount: i32, mana_type: String },
    /// Waterbend cost (Waterbend<N>). Mirrors CostWaterbend.
    /// Pay N generic mana, but you can tap your artifacts and creatures to help (each tapped = {1}).
    Waterbend { amount: i32 },
    /// Choose one or more colors as a cost. Mirrors CostChooseColor.
    ChooseColor(i32),
    /// Choose a creature type as a cost. Mirrors CostChooseCreatureType.
    ChooseCreatureType(i32),
    /// Flip one or more coins as a cost. Mirrors CostFlipCoin.
    FlipCoin(i32),
    /// Roll dice as a cost. Mirrors CostRollDice.
    RollDice {
        amount: i32,
        sides: i32,
        result_svar: String,
    },
    /// Exile spells from stack as a cost. Mirrors CostExileFromStack.
    ExileFromStack { amount: i32, type_filter: String },
    /// Collect evidence N (exile cards from your graveyard with total MV >= N).
    CollectEvidence(i32),
    /// Forage: exile 3 from your graveyard or sacrifice a Food.
    Forage,
    /// Put card(s) to library from a zone as a cost. Mirrors CostPutCardToLib.
    PutCardToLib {
        amount: i32,
        lib_pos: i32,
        type_filter: String,
        from: ZoneType,
        same_zone: bool,
    },
    /// Enlist another creature as a cost. Mirrors CostEnlist.
    Enlist { amount: i32, type_filter: String },
    /// Promise gift to an opponent as a cost. Mirrors CostPromiseGift.
    PromiseGift,
    /// Reveal previously chosen player/type as a cost. Mirrors CostRevealChosen.
    RevealChosen { reveal_type: String },
    /// Behold (reveal from hand/battlefield), optionally exile revealed cards.
    Behold {
        amount: i32,
        type_filter: String,
        exile: bool,
    },
    /// Blight N = put N -1/-1 counters on creature(s) you control.
    Blight(i32),
    /// Exile from battlefield or graveyard as a combined cost (craft).
    ExileCtrlOrGrave { amount: i32, type_filter: String },
}

impl CostPart {
    /// Payment ordering — mirrors Java's CostPart.paymentOrder.
    /// Lower numbers are paid first.
    fn payment_order(&self) -> i32 {
        match self {
            CostPart::Tap => -1,
            CostPart::Untap => 20,
            CostPart::Mana(_) => 0,
            CostPart::PayEnergy(_) => 7,
            CostPart::PayShards(_) => 7,
            CostPart::SubCounter { .. } => 8,
            CostPart::AddCounter { .. } => 6,
            CostPart::PayLife(_) => 7,
            CostPart::DamageYou(_) => 8,
            CostPart::GainLife(_) => 5,
            CostPart::Reveal { from, .. } => match from {
                RevealFrom::Hand => 5,
                RevealFrom::HandOrBattlefield => 5,
                _ => -1,
            },
            CostPart::Draw(_) => 20,
            CostPart::Mill(_) => 20,
            CostPart::Discard { .. } => 10,
            CostPart::Sacrifice { .. } => 15,
            CostPart::Exile { from, .. } => {
                if *from == ZoneType::Library {
                    20
                } else {
                    15
                }
            }
            CostPart::ExileFromAnyGrave { .. } => 15,
            CostPart::ExileFromSameGrave { .. } => 15,
            CostPart::Return { .. } => 10,
            CostPart::TapType { .. } => 18,
            CostPart::UntapType { .. } => 18,
            CostPart::GainControl { .. } => 8,
            CostPart::RemoveAnyCounter { .. } => 8,
            CostPart::Unattach => 5,
            CostPart::ExiledMoveToGrave { .. } => 15,
            CostPart::AddMana { .. } => 5,
            CostPart::Waterbend { .. } => 0,
            CostPart::Exert { .. } => 5,
            CostPart::ChooseColor(_) => 8,
            CostPart::ChooseCreatureType(_) => 5,
            CostPart::FlipCoin(_) => 22,
            CostPart::RollDice { .. } => 5,
            CostPart::ExileFromStack { .. } => 15,
            CostPart::CollectEvidence(_) => 15,
            CostPart::Forage => 5,
            CostPart::PutCardToLib { .. } => 10,
            CostPart::Enlist { .. } => 5,
            CostPart::PromiseGift => -1,
            CostPart::RevealChosen { .. } => 20,
            CostPart::Behold { .. } => 5,
            CostPart::Blight(_) => 6,
            CostPart::ExileCtrlOrGrave { .. } => 15,
        }
    }
}

/// The complete cost to activate an ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub parts: Vec<CostPart>,
    pub has_tap: bool,
    pub mandatory: bool,
}

impl Cost {
    pub fn is_zero_cost(&self) -> bool {
        self.parts.is_empty()
            || (self.parts.len() == 1
                && matches!(&self.parts[0], CostPart::Mana(mana) if mana.is_zero()))
    }
}

/// Parse a Cost$ value from the DSL.
///
/// Examples:
/// - `"T"` → tap
/// - `"1 G"` → mana cost {1}{G}
/// - `"T 1 G"` → tap + mana
/// - `"Sac<1/CARDNAME>"` → sacrifice self
/// - `"PayLife<3>"` → pay 3 life
/// - `"Exile<1/Creature>"` → exile a creature from battlefield
/// - `"ExileFromHand<1/Card>"` → exile from hand
/// - `"Return<1/CARDNAME>"` → return self to hand
/// - `"tapXType<1/Creature>"` → tap a creature
/// - `"AddCounter<1/LOYALTY>"` → add a loyalty counter
/// - `"SubCounter<1/LOYALTY/CARDNAME>"` → remove loyalty counter
pub fn parse_cost(raw: &str) -> Cost {
    let tokens = split_cost_tokens(raw);
    let mut parts = Vec::new();
    let mut has_tap = false;
    let mut mandatory = false;
    let mut mana_tokens: Vec<&str> = Vec::new();

    for token in &tokens {
        match cost_parser::parse_cost_token(token) {
            cost_parser::TokenResult::Part(part) => parts.push(part),
            cost_parser::TokenResult::Tap => {
                parts.push(CostPart::Tap);
                has_tap = true;
            }
            cost_parser::TokenResult::Mandatory => {
                mandatory = true;
            }
            cost_parser::TokenResult::Mana => {
                mana_tokens.push(token);
            }
        }
    }

    // If we have mana tokens, combine them into a ManaCost
    if !mana_tokens.is_empty() {
        let mana_str = mana_tokens.join(" ");
        let mana_cost = ManaCost::parse(&mana_str);
        if mana_cost.cmc() > 0 || !mana_str.is_empty() {
            parts.push(CostPart::Mana(mana_cost));
        }
    }

    // Sort by payment order
    parts.sort_by_key(|p| p.payment_order());

    Cost {
        parts,
        has_tap,
        mandatory,
    }
}

/// Parse `"amount/filter"` inner content, returning (amount, filter).
/// If there's no slash, defaults to amount=1 and filter=inner.
pub(super) fn parse_amount_filter(inner: &str) -> (i32, String) {
    if let Some(slash_idx) = inner.find('/') {
        let amt = parse_i32_or_x(&inner[..slash_idx], 1);
        // Strip any trailing description (second slash)
        let rest = &inner[slash_idx + 1..];
        let filter = if let Some(desc_idx) = rest.find('/') {
            rest[..desc_idx].to_string()
        } else {
            rest.to_string()
        };
        (amt, filter)
    } else {
        (1, inner.to_string())
    }
}

pub(super) fn parse_amount_filter_dynamic(inner: &str) -> (i32, String) {
    if let Some(slash_idx) = inner.find('/') {
        let amt = parse_i32_or_x(&inner[..slash_idx], 1);
        let rest = &inner[slash_idx + 1..];
        let filter = if let Some(desc_idx) = rest.find('/') {
            rest[..desc_idx].to_string()
        } else {
            rest.to_string()
        };
        (amt, filter)
    } else {
        (parse_i32_or_x(inner, 1), inner.to_string())
    }
}

/// Split cost string on spaces, keeping `<...>` groups together.
fn split_cost_tokens(raw: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let bytes = raw.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b' ' if depth == 0 => {
                let token = raw[start..i].trim();
                if !token.is_empty() {
                    tokens.push(token);
                }
                start = i + 1;
            }
            _ => {
                // This is a character-parsing loop, not a semantic dispatch
                // No warning needed here — we're just walking through characters
            }
        }
        i += 1;
    }
    // Last token
    let token = raw[start..].trim();
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

/// Check if a card matches a type filter string.
/// Thin wrapper around `matches_change_type` for use by individual cost modules.
pub fn matches_type_filter(game: &GameState, cid: CardId, type_filter: &str) -> bool {
    matches_change_type(game.card(cid), type_filter, &[])
}

/// Find valid sacrifice targets on the battlefield for a player, filtered by type.
/// Mirrors Java's `CostSacrifice.getMaxAmountX()` + `CardLists.getValidCards()`.
pub fn get_sacrifice_targets(game: &GameState, player: PlayerId, type_filter: &str) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| matches_change_type(game.card(cid), type_filter, &[]))
        .collect()
}

/// Find valid exile/return targets in a given zone for a player, filtered by type.
pub fn get_zone_targets(
    game: &GameState,
    player: PlayerId,
    zone: ZoneType,
    type_filter: &str,
) -> Vec<CardId> {
    game.cards_in_zone(zone, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| {
            if type_filter == "Card" || type_filter.is_empty() {
                true
            } else {
                matches_change_type(game.card(cid), type_filter, &[])
            }
        })
        .collect()
}

/// Find cards in exile across all players matching a type filter.
/// Used by ExiledMoveToGrave and can_pay checks.
pub fn get_exiled_targets(game: &GameState, type_filter: &str) -> Vec<CardId> {
    game.players
        .iter()
        .flat_map(|p| game.cards_in_zone(ZoneType::Exile, p.id).to_vec())
        .filter(|&cid| {
            type_filter == "Card"
                || type_filter.is_empty()
                || matches_change_type(game.card(cid), type_filter, &[])
        })
        .collect()
}

/// Find valid tap-type targets (untapped permanents matching filter, excluding source).
pub fn get_tap_type_targets(
    game: &GameState,
    player: PlayerId,
    type_filter: &str,
    exclude: CardId,
) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| {
            if cid == exclude {
                return false;
            }
            let card = game.card(cid);
            if card.tapped {
                return false;
            }
            if type_filter == "Card" || type_filter.is_empty() {
                true
            } else {
                matches_change_type(card, type_filter, &[])
            }
        })
        .collect()
}

/// Find cards available to enlist: untapped, non-attacking, non-summoning-sick creatures you control.
pub fn get_enlist_targets(game: &GameState, player: PlayerId) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| {
            let c = game.card(cid);
            // Mirror Java CostEnlist.getCardsForEnlisting():
            // c.canTap() && !c.isSick() && !c.isAttacking()
            // where isSick() is false for creatures with haste.
            c.is_creature()
                && !c.tapped
                && !c.phased_out
                && (!c.summoning_sick || c.has_haste())
                && c.attacking_player.is_none()
        })
        .collect()
}

pub fn matches_exile_from_stack_filter(
    game: &GameState,
    card_id: CardId,
    player: PlayerId,
    type_filter: &str,
) -> bool {
    if type_filter == "All" || type_filter.is_empty() {
        return true;
    }
    let card = game.card(card_id);
    for clause in type_filter.split(';') {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        let normalized = normalize_stack_clause_for_valid_cards(clause);
        if matches_valid_cards(card, &normalized, player) {
            return true;
        }
    }
    false
}

fn is_valid_cards_type_token(token: &str) -> bool {
    matches!(
        token,
        "Card"
            | "Permanent"
            | "Creature"
            | "Land"
            | "Artifact"
            | "Enchantment"
            | "Planeswalker"
            | "Instant"
            | "Sorcery"
            | "Plains"
            | "Island"
            | "Swamp"
            | "Mountain"
            | "Forest"
    )
}

fn normalize_stack_clause_for_valid_cards(clause: &str) -> String {
    let mut tokens: Vec<&str> = clause
        .split(['.', '+'])
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("Spell"))
        .collect();

    if tokens.is_empty() {
        return "Card".to_string();
    }

    let type_idx = tokens
        .iter()
        .position(|t| is_valid_cards_type_token(t))
        .unwrap_or(usize::MAX);

    if type_idx == usize::MAX {
        let mut out = String::from("Card");
        for t in tokens.drain(..) {
            out.push('.');
            out.push_str(t);
        }
        return out;
    }

    let type_part = tokens[type_idx].to_string();
    let mut qualifiers: Vec<&str> = Vec::with_capacity(tokens.len().saturating_sub(1));
    qualifiers.extend(tokens[..type_idx].iter().copied());
    qualifiers.extend(tokens[type_idx + 1..].iter().copied());

    if qualifiers.is_empty() {
        type_part
    } else {
        format!("{}.{}", type_part, qualifiers.join("."))
    }
}

pub fn strip_exile_type_modifiers(type_filter: &str) -> String {
    let mut t = type_filter.to_string();
    if t.contains("FromTopGrave") {
        t = t.replace("FromTopGrave", "");
    }
    if let Some((left, _)) = t.split_once("+withTotalCMCEQ") {
        t = left.to_string();
    }
    if let Some((left, _)) = t.split_once("+withTotalCMCGE") {
        t = left.to_string();
    }
    if t.contains("+withSharedCardType") {
        t = t.replace("+withSharedCardType", "");
    }
    if let Some((left, _)) = t.split_once("+withTypesGE") {
        t = left.to_string();
    }
    t
}

pub fn normalize_exile_base_filter(type_filter: &str) -> String {
    let t = strip_exile_type_modifiers(type_filter);
    if t.is_empty() || t.eq_ignore_ascii_case("All") || t.contains('X') {
        "Card".to_string()
    } else {
        t
    }
}

fn parse_exile_total_cmc_eq(type_filter: &str) -> Option<&str> {
    type_filter
        .split_once("+withTotalCMCEQ")
        .map(|(_, rhs)| rhs.trim())
}

fn parse_exile_total_cmc_ge(type_filter: &str) -> Option<&str> {
    type_filter
        .split_once("+withTotalCMCGE")
        .map(|(_, rhs)| rhs.trim())
}

fn parse_exile_types_ge(type_filter: &str) -> Option<i32> {
    type_filter
        .split_once("+withTypesGE")
        .and_then(|(_, rhs)| rhs.trim().parse::<i32>().ok())
}

fn exile_requires_shared_card_type(type_filter: &str) -> bool {
    type_filter.contains("+withSharedCardType")
}

fn reveal_candidates(
    game: &GameState,
    player: PlayerId,
    source: CardId,
    type_filter: &str,
    from: &RevealFrom,
) -> Vec<CardId> {
    let mut cards: Vec<CardId> = match from {
        RevealFrom::Hand => game.cards_in_zone(ZoneType::Hand, player).to_vec(),
        RevealFrom::Exile => game.cards_in_zone(ZoneType::Exile, player).to_vec(),
        RevealFrom::HandOrBattlefield => {
            let mut v = game.cards_in_zone(ZoneType::Hand, player).to_vec();
            v.extend(
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied(),
            );
            v
        }
        RevealFrom::All => {
            let mut v = game.cards_in_zone(ZoneType::Hand, player).to_vec();
            v.extend(
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied(),
            );
            v.extend(
                game.cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .copied(),
            );
            v.extend(
                game.cards_in_zone(ZoneType::Library, player)
                    .iter()
                    .copied(),
            );
            v.extend(game.cards_in_zone(ZoneType::Exile, player).iter().copied());
            v
        }
    };

    // Spell costs can't pay reveal from the source card itself while in hand.
    if matches!(
        from,
        RevealFrom::Hand | RevealFrom::HandOrBattlefield | RevealFrom::All
    ) && game.card(source).zone == ZoneType::Hand
    {
        cards.retain(|&cid| cid != source);
    }

    if type_filter == "Card" || type_filter.is_empty() || type_filter == "Hand" {
        return cards;
    }

    cards
        .into_iter()
        .filter(|&cid| matches_change_type(game.card(cid), type_filter, &[]))
        .collect()
}

/// Check if a cost can be paid by the given player for the given source card.
/// `available_mana` is the total mana available (pool + untapped sources).
pub fn can_pay(
    cost: &Cost,
    game: &GameState,
    available_mana: &ManaPool,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, Some(available_mana), source, player, None)
}

/// Java parity helper: canPay(cost, ability-context).
pub fn can_pay_with_ability(
    cost: &Cost,
    game: &GameState,
    available_mana: &ManaPool,
    source: CardId,
    player: PlayerId,
    ability: Option<&SpellAbility>,
) -> bool {
    can_pay_inner(cost, game, Some(available_mana), source, player, ability)
}

/// Check if a cost can be paid ignoring mana requirements.
/// Used for mana ability availability checks (to avoid circular dependency).
pub fn can_pay_ignoring_mana(
    cost: &Cost,
    game: &GameState,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, None, source, player, None)
}

/// Check if a cost can be paid ignoring mana requirements, for a spell.
/// Passes a minimal SpellAbility with `is_spell = true` so that CantSacrifice
/// checks (e.g. Yasharn) can properly evaluate `ValidCause$ Spell` restrictions.
pub fn can_pay_ignoring_mana_for_spell(
    cost: &Cost,
    game: &GameState,
    source: CardId,
    player: PlayerId,
) -> bool {
    let stub = SpellAbility {
        is_spell: true,
        source: Some(source),
        activating_player: player,
        api: None,
        targeting_player: None,
        ability_text: String::new(),
        params: crate::parsing::Params::default(),
        target_restrictions: None,
        target_chosen: crate::spellability::TargetChoices::default(),
        pay_costs: None,
        sub_ability: None,
        is_trigger: false,
        is_activated: false,
        trigger_source: None,
        trigger_index: None,
        alt_cost: None,
        kicked: false,
        buyback_paid: false,
        overloaded: false,
        is_copy: false,
        kick_count: 0,
        replicate_count: 0,
        optional_generic_cost_paid: false,
        trigger_remembered_amount: 0,
        x_mana_cost_paid: 0,
        discarded_cost_cards: Vec::new(),
        change_zone_table: None,
        damage_map: None,
        prevent_map: None,
    };
    can_pay_inner(cost, game, None, source, player, Some(&stub))
}

/// Shared implementation for cost payability checks.
/// When `available_mana` is None, mana costs are skipped.
fn can_pay_inner(
    cost: &Cost,
    game: &GameState,
    available_mana: Option<&ManaPool>,
    source: CardId,
    player: PlayerId,
    ability: Option<&SpellAbility>,
) -> bool {
    let card = game.card(source);
    let static_source_cards = static_ability_source_cards(game);

    for part in &cost.parts {
        match part {
            CostPart::Tap => {
                // Mirrors Java's CostTap.canPay() → source.canTap() && !source.isAbilitySick()
                if card.tapped || card.phased_out {
                    return false;
                }
                if card.is_creature() && card.summoning_sick && !card.has_haste() {
                    return false;
                }
            }
            CostPart::Untap => {
                // Mirrors Java's CostUntap.canPay() → source.canUntap() && !source.isAbilitySick()
                if !card.tapped || card.phased_out {
                    return false;
                }
                if card.is_creature() && card.summoning_sick && !card.has_haste() {
                    return false;
                }
                // Stun counters prevent untapping
                if *card
                    .counters
                    .get(&CounterType::Named("STUN".to_string()))
                    .unwrap_or(&0)
                    > 0
                {
                    return false;
                }
            }
            CostPart::Mana(mana_cost) => {
                if let Some(pool) = available_mana {
                    if !pool.can_pay(mana_cost) {
                        return false;
                    }
                }
            }
            CostPart::PayLife(amount) => {
                // Static abilities (Platinum Emperion, etc.) can prevent life payment.
                if cant_pay_life(game, player, true, None) {
                    return false;
                }
                // Player needs at least `amount` life — paying lethal life is legal in MTG.
                if game.player(player).life < *amount {
                    return false;
                }
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Battlefield {
                        return false;
                    }
                    // Mirrors Java's canBeSacrificedBy()
                    if cant_sacrifice(&static_source_cards, card, ability, true) {
                        return false;
                    }
                } else if type_filter.eq_ignore_ascii_case("All") {
                    // Java: "All" requires every matching card canBeSacrificedBy
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    if targets.iter().any(|&cid| {
                        cant_sacrifice(&static_source_cards, game.card(cid), ability, true)
                    }) {
                        return false;
                    }
                } else {
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    let valid = targets
                        .iter()
                        .filter(|&&cid| {
                            !cant_sacrifice(&static_source_cards, game.card(cid), ability, true)
                        })
                        .count() as i32;
                    if valid < *amount {
                        return false;
                    }
                }
            }
            CostPart::Discard {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    // Discard the source card itself — it must be in hand.
                    if card.zone != ZoneType::Hand {
                        return false;
                    }
                } else if type_filter == "Card" || type_filter.is_empty() {
                    // Any card — just need enough cards in hand.
                    let mut hand_size = game.cards_in_zone(ZoneType::Hand, player).len() as i32;
                    if card.zone == ZoneType::Hand && card.owner == player {
                        hand_size -= 1;
                    }
                    if hand_size < *amount {
                        return false;
                    }
                } else {
                    // Type-filtered discard — count matching cards in hand.
                    // Mirrors Java CostDiscard.getMaxAmountX() filtering by getType().
                    let mut matching = game
                        .cards_in_zone(ZoneType::Hand, player)
                        .iter()
                        .filter(|&&cid| matches_change_type(game.card(cid), type_filter, &[]))
                        .count() as i32;
                    if card.zone == ZoneType::Hand
                        && card.owner == player
                        && matches_change_type(card, type_filter, &[])
                    {
                        matching -= 1;
                    }
                    if matching < *amount {
                        return false;
                    }
                }
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                // Mirrors Java's CostRemoveCounter.canPay(): !source.isPhasedOut()
                if card.zone != ZoneType::Battlefield || card.phased_out {
                    return false;
                }
                if card.counter_count(counter_type) < *amount {
                    return false;
                }
            }
            CostPart::AddCounter { .. } => {
                // AddCounter (put counter on source) is always payable if source is on battlefield.
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
            }
            CostPart::Exile {
                amount,
                type_filter,
                from,
            } => {
                if type_filter == "All" {
                    continue;
                }
                if type_filter == "CARDNAME" || type_filter == "OriginalHost" {
                    if card.zone != *from {
                        return false;
                    }
                    // Mirrors Java: CantExile static ability check on self-exile
                    if cant_exile(&static_source_cards, card, ability, true) {
                        return false;
                    }
                } else {
                    let base_filter = normalize_exile_base_filter(type_filter);
                    let candidates: Vec<CardId> =
                        get_zone_targets(game, player, *from, &base_filter)
                            .into_iter()
                            .filter(|&cid| {
                                !cant_exile(&static_source_cards, game.card(cid), ability, true)
                            })
                            .collect();
                    let mut available = candidates.len() as i32;
                    if *from == ZoneType::Hand
                        && card.zone == ZoneType::Hand
                        && card.owner == player
                        && matches_change_type(card, &base_filter, &[])
                    {
                        available -= 1;
                    }
                    if let Some(n) = parse_exile_types_ge(type_filter) {
                        let mut unique_types = std::collections::BTreeSet::new();
                        for cid in &candidates {
                            for t in &game.card(*cid).type_line.core_types {
                                unique_types.insert(format!("{:?}", t));
                            }
                        }
                        if (unique_types.len() as i32) < n {
                            return false;
                        }
                    }
                    if let Some(expr) = parse_exile_total_cmc_eq(type_filter) {
                        let target = if expr.eq_ignore_ascii_case("X") {
                            None
                        } else {
                            expr.parse::<i32>().ok()
                        };
                        if let Some(target) = target {
                            let values: Vec<i32> = candidates
                                .iter()
                                .map(|&cid| game.card(cid).mana_cost.cmc() as i32)
                                .collect();
                            if !cmc_can_sum_to(target, &values) {
                                return false;
                            }
                        }
                    }
                    if let Some(expr) = parse_exile_total_cmc_ge(type_filter) {
                        let target = if expr.eq_ignore_ascii_case("X") {
                            None
                        } else {
                            expr.parse::<i32>().ok()
                        };
                        if let Some(target) = target {
                            let total: i32 = candidates
                                .iter()
                                .map(|&cid| game.card(cid).mana_cost.cmc() as i32)
                                .sum();
                            if total < target {
                                return false;
                            }
                        }
                    }
                    if exile_requires_shared_card_type(type_filter) {
                        if available < *amount {
                            return false;
                        }
                        let mut has_pair = false;
                        for &a in &candidates {
                            for &b in &candidates {
                                if a != b && shares_card_type(game, a, b) {
                                    has_pair = true;
                                    break;
                                }
                            }
                            if has_pair {
                                break;
                            }
                        }
                        if !has_pair {
                            return false;
                        }
                    }
                    if available < *amount {
                        return false;
                    }
                }
            }
            CostPart::Return {
                amount,
                type_filter,
            } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Battlefield {
                        return false;
                    }
                } else {
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            CostPart::TapType {
                amount,
                type_filter,
                min_total_power,
            } => {
                let targets = get_tap_type_targets(game, player, type_filter, source);
                if let Some(power_threshold) = min_total_power {
                    // Crew: check total power of all valid targets >= threshold
                    let total_power: i32 = targets.iter().map(|&cid| game.card(cid).power()).sum();
                    if total_power < *power_threshold {
                        return false;
                    }
                } else if (targets.len() as i32) < *amount {
                    return false;
                }
            }
            CostPart::UntapType {
                amount,
                type_filter,
            } => {
                // Untap tapped permanents matching type
                let count = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        cid != source
                            && game.card(cid).tapped
                            && (type_filter == "Card"
                                || type_filter.is_empty()
                                || matches_change_type(game.card(cid), type_filter, &[]))
                    })
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::PayEnergy(amount) => {
                if game.player(player).energy_counters < *amount {
                    return false;
                }
            }
            CostPart::PayShards(amount) => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if game.player(player).mana_shards < resolved_amount {
                    return false;
                }
            }
            CostPart::DamageYou(_) => {
                // Mirrors Java CostDamage.canPay() — always returns true.
                // The player may die as a state-based action after payment; that's legal.
            }
            CostPart::Draw(amount) => {
                // Mirrors Java's CostDraw.canPay() → p.canDrawAmount(c)
                let resolved = resolve_dynamic_amount(game, source, player, *amount);
                let allowed = crate::staticability::static_ability_cant_draw::can_draw_amount(
                    game, player, resolved,
                );
                if allowed < resolved {
                    return false;
                }
            }
            CostPart::Mill(amount) => {
                // Mirrors Java's CostMill.canPay(): amount < library.size() (strict <)
                let resolved = resolve_dynamic_amount(game, source, player, *amount);
                let lib_size = game.zone(ZoneType::Library, player).len() as i32;
                if lib_size <= resolved {
                    return false;
                }
            }
            CostPart::Reveal {
                amount,
                type_filter,
                from,
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if type_filter == "Hand" {
                    continue;
                }
                if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                    let src_zone = game.card(source).zone;
                    let in_zone = match from {
                        RevealFrom::Hand => src_zone == ZoneType::Hand,
                        RevealFrom::Exile => src_zone == ZoneType::Exile,
                        RevealFrom::HandOrBattlefield => {
                            src_zone == ZoneType::Hand || src_zone == ZoneType::Battlefield
                        }
                        RevealFrom::All => true,
                    };
                    if !in_zone {
                        return false;
                    }
                    continue;
                }
                let candidates = reveal_candidates(game, player, source, type_filter, from);
                if type_filter == "SameColor" {
                    let mut ok = false;
                    for &cid in &candidates {
                        let color = game.card(cid).color;
                        let count = candidates
                            .iter()
                            .filter(|&&other| game.card(other).color.shares_color_with(color))
                            .count() as i32;
                        if count >= resolved_amount {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        return false;
                    }
                } else if (candidates.len() as i32) < resolved_amount {
                    return false;
                }
            }
            CostPart::Exert {
                amount,
                type_filter,
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                    if resolved_amount > 1 {
                        return false;
                    }
                } else {
                    let count = game
                        .cards_in_zone(ZoneType::Battlefield, player)
                        .iter()
                        .filter(|&&cid| matches_change_type(game.card(cid), type_filter, &[]))
                        .count() as i32;
                    if count < resolved_amount {
                        return false;
                    }
                }
            }
            CostPart::GainLife(_) => {
                // Mirrors Java's CostGainLife.canPay() → opponent.canGainLife()
                let opponent = game.opponent_of(player);
                if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_gain_life(
                    game, opponent,
                ) {
                    return false;
                }
            }
            CostPart::GainControl {
                amount,
                type_filter,
            } => {
                // Scan all players' battlefields — can gain control of any matching permanent.
                // Mirrors Java CostGainControl.canPay() which calls getCardsIn(ZoneType.Battlefield)
                // across all players.
                let count = game
                    .players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(ZoneType::Battlefield, p.id))
                    .filter(|&&cid| matches_change_type(game.card(cid), type_filter, &[]))
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::RemoveAnyCounter {
                amount,
                type_filter,
                counter_type,
            } => {
                // Sum counters of the given type (or any type) across all matching permanents
                let total: i32 = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        type_filter == "Permanent"
                            || type_filter.is_empty()
                            || matches_change_type(game.card(cid), type_filter, &[])
                    })
                    .map(|&cid| {
                        let c = game.card(cid);
                        match counter_type {
                            Some(ct) => c.counter_count(ct),
                            None => c.counters.values().sum(),
                        }
                    })
                    .sum();
                if total < *amount {
                    return false;
                }
            }
            CostPart::Unattach => {
                // Source must be on the battlefield and currently equipping something
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
                if card.attached_to.is_none() {
                    return false;
                }
            }
            CostPart::ExileFromAnyGrave {
                amount,
                type_filter,
            } => {
                let base_filter = normalize_exile_base_filter(type_filter);
                // Cards in ANY player's graveyard matching filter.
                let count = game
                    .players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(ZoneType::Graveyard, p.id))
                    .filter(|&&cid| {
                        (base_filter == "Card"
                            || base_filter.is_empty()
                            || matches_change_type(game.card(cid), &base_filter, &[]))
                            && !cant_exile(&static_source_cards, game.card(cid), ability, true)
                    })
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::ExileFromSameGrave {
                amount,
                type_filter,
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                let base_filter = normalize_exile_base_filter(type_filter);
                let mut by_owner: std::collections::HashMap<PlayerId, i32> =
                    std::collections::HashMap::new();
                for p in &game.players {
                    for &cid in game.cards_in_zone(ZoneType::Graveyard, p.id) {
                        if base_filter == "Card"
                            || base_filter.is_empty()
                            || matches_change_type(game.card(cid), &base_filter, &[])
                        {
                            if cant_exile(&static_source_cards, game.card(cid), ability, true) {
                                continue;
                            }
                            let owner = game.card(cid).owner;
                            *by_owner.entry(owner).or_insert(0) += 1;
                        }
                    }
                }
                if by_owner.values().all(|&v| v < resolved_amount) {
                    return false;
                }
            }
            CostPart::ExiledMoveToGrave {
                amount,
                type_filter,
            } => {
                // Count cards in exile across all players matching the filter
                let exiled = get_exiled_targets(game, type_filter).len() as i32;
                if exiled < *amount {
                    return false;
                }
            }
            CostPart::AddMana { .. } => {
                // Adding mana to pool is always payable (mirrors Java CostAddMana.canPay)
            }
            CostPart::Waterbend { amount } => {
                // Payable if available mana + tappable artifacts/creatures >= amount
                let pool_total = available_mana.map_or(0, |p| p.total());
                let tappable_count = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        let c = game.card(cid);
                        !c.tapped && cid != source && (c.is_creature() || c.type_line.is_artifact())
                    })
                    .count() as i32;
                if pool_total + tappable_count < *amount {
                    return false;
                }
            }
            CostPart::ChooseColor(_) => {}
            CostPart::ChooseCreatureType(_) => {}
            CostPart::FlipCoin(_) => {}
            CostPart::RollDice { .. } => {}
            CostPart::PromiseGift => {}
            CostPart::RevealChosen { reveal_type } => {
                let source_card = game.card(source);
                if reveal_type.eq_ignore_ascii_case("Player") {
                    if source_card.chosen_player.is_none() {
                        return false;
                    }
                    if source_card
                        .chosen_player_controller
                        .is_some_and(|pid| pid != player)
                    {
                        return false;
                    }
                } else if reveal_type.eq_ignore_ascii_case("Type") {
                    if source_card.chosen_type.is_none() {
                        return false;
                    }
                    if source_card
                        .chosen_type_controller
                        .is_some_and(|pid| pid != player)
                    {
                        return false;
                    }
                }
            }
            CostPart::CollectEvidence(amount) => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                let total_mv: i32 = game
                    .cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .filter(|&&cid| {
                        !cant_exile(&static_source_cards, game.card(cid), ability, true)
                    })
                    .map(|&cid| game.card(cid).mana_cost.cmc() as i32)
                    .sum();
                if total_mv < resolved_amount {
                    return false;
                }
            }
            CostPart::Forage => {
                let gy_count = game
                    .cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .filter(|&&cid| {
                        !cant_exile(&static_source_cards, game.card(cid), ability, true)
                    })
                    .count() as i32;
                let has_food = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .any(|&cid| {
                        game.card(cid).type_line.has_subtype("Food")
                            && !cant_sacrifice(&static_source_cards, game.card(cid), ability, true)
                    });
                if gy_count < 3 && !has_food {
                    return false;
                }
            }
            CostPart::ExileFromStack {
                amount,
                type_filter,
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if type_filter == "All" {
                    continue;
                }
                let count = game
                    .stack
                    .iter()
                    .filter(|e| e.spell_ability.is_spell)
                    .filter_map(|e| e.spell_ability.source)
                    .filter(|&cid| matches_exile_from_stack_filter(game, cid, player, type_filter))
                    .count() as i32;
                if count < resolved_amount {
                    return false;
                }
            }
            CostPart::PutCardToLib {
                amount,
                type_filter,
                from,
                same_zone,
                ..
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                    if *same_zone {
                        let in_zone = game
                            .players
                            .iter()
                            .any(|p| game.cards_in_zone(*from, p.id).contains(&source));
                        if !in_zone {
                            return false;
                        }
                    } else if game.card(source).zone != *from {
                        return false;
                    }
                    continue;
                }
                if *same_zone {
                    let pool: Vec<CardId> = game
                        .players
                        .iter()
                        .flat_map(|p| game.cards_in_zone(*from, p.id).to_vec())
                        .filter(|&cid| {
                            type_filter == "Card"
                                || type_filter.is_empty()
                                || matches_change_type(game.card(cid), type_filter, &[])
                        })
                        .collect();
                    let mut by_controller: std::collections::HashMap<PlayerId, i32> =
                        std::collections::HashMap::new();
                    for cid in pool {
                        let ctrl = game.card(cid).controller;
                        *by_controller.entry(ctrl).or_insert(0) += 1;
                    }
                    if by_controller.values().all(|&v| v < resolved_amount) {
                        return false;
                    }
                } else {
                    let count = get_zone_targets(game, player, *from, type_filter).len() as i32;
                    if count < resolved_amount {
                        return false;
                    }
                }
            }
            CostPart::Enlist { .. } => {
                let valid = get_enlist_targets(game, player);
                if valid.is_empty() {
                    return false;
                }
            }
            CostPart::Behold {
                amount,
                type_filter,
                ..
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                if type_filter.ends_with("ChosenType") {
                    let mut cards = game.cards_in_zone(ZoneType::Hand, player).to_vec();
                    cards.extend(
                        game.cards_in_zone(ZoneType::Battlefield, player)
                            .iter()
                            .copied(),
                    );
                    let mut ok = false;
                    for &cid in &cards {
                        let shared = cards
                            .iter()
                            .filter(|&&other| shares_creature_type(game, cid, other))
                            .count() as i32;
                        if shared >= resolved_amount {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        return false;
                    }
                    continue;
                }
                let mut count = 0i32;
                for &cid in game.cards_in_zone(ZoneType::Hand, player) {
                    // While casting a spell from hand, the source card itself can't be
                    // revealed/exiled to satisfy its own Behold additional cost.
                    if cid == source {
                        continue;
                    }
                    if type_filter == "Card"
                        || type_filter.is_empty()
                        || matches_change_type(game.card(cid), type_filter, &[])
                    {
                        count += 1;
                    }
                }
                for &cid in game.cards_in_zone(ZoneType::Battlefield, player) {
                    if type_filter == "Card"
                        || type_filter.is_empty()
                        || matches_change_type(game.card(cid), type_filter, &[])
                    {
                        count += 1;
                    }
                }
                if count < resolved_amount {
                    return false;
                }
            }
            CostPart::Blight(amount) => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                let battlefield_cards: Vec<_> = game
                    .players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(ZoneType::Battlefield, p.id))
                    .map(|&cid| game.card(cid).clone())
                    .collect();
                let creature_count = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        let c = game.card(cid);
                        c.is_creature()
                            && !any_cant_put_counter_on_card(
                                &battlefield_cards,
                                c,
                                &CounterType::M1M1,
                            )
                    })
                    .count() as i32;
                if creature_count < resolved_amount {
                    return false;
                }
            }
            CostPart::ExileCtrlOrGrave {
                amount,
                type_filter,
            } => {
                let resolved_amount = resolve_dynamic_amount(game, source, player, *amount);
                let base_filter = normalize_exile_base_filter(type_filter);
                let bf = get_zone_targets(game, player, ZoneType::Battlefield, &base_filter)
                    .into_iter()
                    .filter(|&cid| !cant_exile(&static_source_cards, game.card(cid), ability, true))
                    .count();
                let gy = get_zone_targets(game, player, ZoneType::Graveyard, &base_filter)
                    .into_iter()
                    .filter(|&cid| !cant_exile(&static_source_cards, game.card(cid), ability, true))
                    .count();
                if ((bf + gy) as i32) < resolved_amount {
                    return false;
                }
            }
        }
    }

    true
}

pub fn static_ability_source_cards(game: &GameState) -> Vec<Card> {
    use std::collections::HashSet;

    let mut ids: HashSet<CardId> = HashSet::new();
    for p in &game.players {
        for &zone in &[
            ZoneType::Battlefield,
            ZoneType::Graveyard,
            ZoneType::Exile,
            ZoneType::Command,
        ] {
            for &cid in game.cards_in_zone(zone, p.id) {
                ids.insert(cid);
            }
        }
    }
    for entry in game.stack.iter() {
        if let Some(cid) = entry.spell_ability.source {
            ids.insert(cid);
        }
    }

    ids.into_iter().map(|cid| game.card(cid).clone()).collect()
}

fn shares_creature_type(game: &GameState, a: CardId, b: CardId) -> bool {
    let ca = game.card(a);
    let cb = game.card(b);
    if !ca.is_creature() || !cb.is_creature() {
        return false;
    }
    ca.type_line
        .subtypes
        .iter()
        .any(|st| cb.type_line.has_subtype(st))
}

fn shares_card_type(game: &GameState, a: CardId, b: CardId) -> bool {
    let ca = game.card(a);
    let cb = game.card(b);
    ca.type_line
        .core_types
        .iter()
        .any(|t| cb.type_line.core_types.contains(t))
}

fn cmc_can_sum_to(target: i32, values: &[i32]) -> bool {
    if target < 0 {
        return false;
    }
    let mut reachable: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    reachable.insert(0);
    for &v in values {
        if v < 0 {
            continue;
        }
        let mut next = reachable.clone();
        for &r in &reachable {
            let nv = r + v;
            if nv <= target {
                next.insert(nv);
            }
        }
        reachable = next;
        if reachable.contains(&target) {
            return true;
        }
    }
    reachable.contains(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tap_only() {
        let cost = parse_cost("T");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::Tap));
    }

    #[test]
    fn parse_mana_only() {
        let cost = parse_cost("1 G");
        assert!(!cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Mana(mc) => assert_eq!(mc.cmc(), 2),
            _ => panic!("expected Mana cost part"),
        }
    }

    #[test]
    fn parse_tap_and_mana() {
        let cost = parse_cost("T 1 G");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap should come first (payment_order = -1)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
    }

    #[test]
    fn parse_sacrifice() {
        let cost = parse_cost("Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "CARDNAME");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn parse_pay_life() {
        let cost = parse_cost("PayLife<3>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PayLife(n) => assert_eq!(*n, 3),
            _ => panic!("expected PayLife cost part"),
        }
    }

    #[test]
    fn parse_compound_cost() {
        let cost = parse_cost("T Sac<1/CARDNAME>");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap first (order -1), then sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Sacrifice { .. }));
    }

    #[test]
    fn parse_sacrifice_creature() {
        let cost = parse_cost("B Sac<1/Creature>");
        assert_eq!(cost.parts.len(), 2);
        // Mana (order 0) before Sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Mana(_)));
        match &cost.parts[1] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Creature");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn payment_order_sorting() {
        // PayLife, Tap, Mana, Sacrifice — should sort to: Tap, Mana, PayLife, Sacrifice
        let cost = parse_cost("PayLife<2> T 1 G Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 4);
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
        assert!(matches!(cost.parts[2], CostPart::PayLife(_)));
        assert!(matches!(cost.parts[3], CostPart::Sacrifice { .. }));
    }

    #[test]
    fn parse_exile_from_hand() {
        let cost = parse_cost("ExileFromHand<1/Card>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Exile {
                amount,
                type_filter,
                from,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Card");
                assert_eq!(*from, ZoneType::Hand);
            }
            _ => panic!("expected Exile cost part"),
        }
    }

    #[test]
    fn parse_add_counter() {
        let cost = parse_cost("AddCounter<1/LOYALTY>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::AddCounter {
                amount,
                counter_type,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(*counter_type, CounterType::Loyalty);
            }
            _ => panic!("expected AddCounter cost part"),
        }
    }

    #[test]
    fn parse_return() {
        let cost = parse_cost("Return<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Return {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "CARDNAME");
            }
            _ => panic!("expected Return cost part"),
        }
    }

    #[test]
    fn parse_tap_type() {
        let cost = parse_cost("tapXType<2/Creature>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::TapType {
                amount,
                type_filter,
                min_total_power,
            } => {
                assert_eq!(*amount, 2);
                assert_eq!(type_filter, "Creature");
                assert_eq!(*min_total_power, None);
            }
            _ => panic!("expected TapType cost part"),
        }
    }

    #[test]
    fn parse_tap_type_with_total_power() {
        let cost = parse_cost("tapXType<Any/Creature.Other+withTotalPowerGE{3}>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::TapType {
                amount,
                type_filter,
                min_total_power,
            } => {
                assert_eq!(*amount, 1); // "Any" defaults to 1
                assert_eq!(type_filter, "Creature.Other");
                assert_eq!(*min_total_power, Some(3));
            }
            _ => panic!("expected TapType cost part"),
        }
    }

    #[test]
    fn parse_pay_energy() {
        let cost = parse_cost("PayEnergy<3>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PayEnergy(n) => assert_eq!(*n, 3),
            _ => panic!("expected PayEnergy cost part"),
        }
    }

    #[test]
    fn parse_explicit_mana_token() {
        let cost = parse_cost("Mana<2 G>");
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::Mana(_)));
    }

    #[test]
    fn parse_collect_evidence() {
        let cost = parse_cost("CollectEvidence<6>");
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::CollectEvidence(6)));
    }

    #[test]
    fn parse_forage() {
        let cost = parse_cost("Forage");
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::Forage));
    }

    #[test]
    fn parse_put_card_to_lib_from_grave() {
        let cost = parse_cost("PutCardToLibFromGrave<1/0/Card>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PutCardToLib {
                amount,
                lib_pos,
                type_filter,
                from,
                same_zone,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(*lib_pos, 0);
                assert_eq!(type_filter, "Card");
                assert_eq!(*from, ZoneType::Graveyard);
                assert!(!same_zone);
            }
            _ => panic!("expected PutCardToLib cost part"),
        }
    }

    #[test]
    fn parse_exile_from_stack() {
        let cost = parse_cost("ExileFromStack<1/Spell>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::ExileFromStack {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Spell");
            }
            _ => panic!("expected ExileFromStack cost part"),
        }
    }

    #[test]
    fn parse_exile_ctrl_or_grave() {
        let cost = parse_cost("ExileCtrlOrGrave<2/Artifact>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::ExileCtrlOrGrave {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 2);
                assert_eq!(type_filter, "Artifact");
            }
            _ => panic!("expected ExileCtrlOrGrave cost part"),
        }
    }
}
