//! Shared helpers for Java `CostPart` parity on top of enum-based Rust cost parts.

use std::cmp::Ordering;

use crate::card::CounterType;
use crate::cost::CostPart;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Mirrors Java's `CostPart.payCostFromSource()`.
pub fn pay_cost_from_source(part: &CostPart) -> bool {
    match part {
        CostPart::Sacrifice { type_filter, .. }
        | CostPart::Discard { type_filter, .. }
        | CostPart::Exile { type_filter, .. }
        | CostPart::Return { type_filter, .. }
        | CostPart::TapType { type_filter, .. }
        | CostPart::UntapType { type_filter, .. }
        | CostPart::GainControl { type_filter, .. }
        | CostPart::RemoveAnyCounter { type_filter, .. }
        | CostPart::ExiledMoveToGrave { type_filter, .. }
        | CostPart::ExileFromStack { type_filter, .. }
        | CostPart::PutCardToLib { type_filter, .. }
        | CostPart::Enlist { type_filter, .. }
        | CostPart::Behold { type_filter, .. }
        | CostPart::ExileCtrlOrGrave { type_filter, .. }
        | CostPart::Unattach { type_filter, .. } => {
            type_filter == "CARDNAME" || type_filter == "NICKNAME"
        }
        _ => false,
    }
}

/// Mirrors Java's `CostPart.convertAmount()`.
pub fn convert_amount(part: &CostPart) -> Option<i32> {
    match part {
        CostPart::PayLife(v)
        | CostPart::PayEnergy(v)
        | CostPart::PayShards(v)
        | CostPart::DamageYou(v)
        | CostPart::Draw(v)
        | CostPart::Mill(v)
        | CostPart::GainLife(v)
        | CostPart::CollectEvidence(v)
        | CostPart::ChooseColor(v)
        | CostPart::ChooseCreatureType(v)
        | CostPart::FlipCoin(v)
        | CostPart::Blight(v) => Some(*v),
        CostPart::SubCounter { amount, .. }
        | CostPart::AddCounter { amount, .. }
        | CostPart::Sacrifice { amount, .. }
        | CostPart::Discard { amount, .. }
        | CostPart::Exile { amount, .. }
        | CostPart::ExileFromAnyGrave { amount, .. }
        | CostPart::ExileFromSameGrave { amount, .. }
        | CostPart::Return { amount, .. }
        | CostPart::TapType { amount, .. }
        | CostPart::UntapType { amount, .. }
        | CostPart::GainControl { amount, .. }
        | CostPart::RemoveAnyCounter { amount, .. }
        | CostPart::ExiledMoveToGrave { amount, .. }
        | CostPart::AddMana { amount, .. }
        | CostPart::ExileFromStack { amount, .. }
        | CostPart::PutCardToLib { amount, .. }
        | CostPart::Enlist { amount, .. }
        | CostPart::Behold { amount, .. }
        | CostPart::ExileCtrlOrGrave { amount, .. } => Some(*amount),
        CostPart::Waterbend { amount } => Some(*amount),
        CostPart::Reveal { amount, .. } => Some(*amount),
        CostPart::Exert { amount, .. } => Some(*amount),
        CostPart::RollDice { amount, .. } => Some(*amount),
        _ => None,
    }
}

/// Mirrors Java's `CostPart.refund(Card source)` dispatch.
pub fn refund(game: &mut GameState, source: CardId, player: PlayerId, part: &CostPart) {
    match part {
        CostPart::Tap => crate::cost::cost_tap::refund(game, source),
        CostPart::Untap => crate::cost::cost_untap::refund(game, source),
        CostPart::PayLife(amount) => crate::cost::cost_pay_life::refund(game, player, *amount),
        CostPart::PayEnergy(amount) => crate::cost::cost_pay_energy::refund(game, player, *amount),
        CostPart::PayShards(amount) => crate::cost::cost_pay_shards::refund(game, player, *amount),
        CostPart::SubCounter {
            amount,
            counter_type,
        } => crate::cost::cost_remove_counter::refund(game, source, *amount, counter_type),
        CostPart::AddCounter {
            amount,
            counter_type,
        } => crate::cost::cost_put_counter::refund(game, source, *amount, counter_type),
        CostPart::ChooseColor(_) => crate::cost::cost_choose_color::refund(game, source),
        _ => {}
    }
}

/// Mirrors Java's `CostPart.applyTextChangeEffects(CardTraitBase)`.
///
/// The current Rust `CostPart` representation does not store immutable original
/// type/description fields like Java, so this is currently a no-op parity shim.
pub fn apply_text_change_effects(_part: &mut CostPart, _game: &GameState, _host: CardId) {}

/// Mirrors Java's `CostPart.paymentOrder()`.
pub fn payment_order(part: &CostPart) -> i32 {
    part.payment_order()
}

/// Mirrors Java's `CostPart.copy()`.
pub fn copy(part: &CostPart) -> CostPart {
    part.clone()
}

/// Mirrors Java's `CostPart.getMaxAmountX(...)`.
pub fn get_max_amount_x(
    game: &GameState,
    source: CardId,
    player: PlayerId,
    part: &CostPart,
    _effect: bool,
) -> Option<i32> {
    let resolved = |v: i32| crate::cost::resolve_dynamic_amount(game, source, player, v);
    match part {
        CostPart::PayEnergy(_) => Some(game.player(player).energy_counters),
        CostPart::PayShards(_) => Some(game.player(player).mana_shards),
        CostPart::PayLife(_) => Some(game.player(player).life.max(0)),
        CostPart::SubCounter {
            amount,
            counter_type,
        } => {
            let current = game.card(source).counter_count(counter_type);
            Some(current.min(resolved(*amount)))
        }
        _ => None,
    }
}

/// Mirrors Java's `CostPart.getAbilityAmount(SpellAbility)`.
pub fn get_ability_amount(
    game: &GameState,
    source: CardId,
    player: PlayerId,
    part: &CostPart,
) -> i32 {
    match convert_amount(part) {
        Some(v) => crate::cost::resolve_dynamic_amount(game, source, player, v),
        None => 0,
    }
}

/// Mirrors Java's `CostPart.isReusable()`.
pub fn is_reusable(part: &CostPart) -> bool {
    match part {
        CostPart::Tap
        | CostPart::Untap
        | CostPart::Mana { .. }
        | CostPart::Reveal { .. }
        | CostPart::TapType { .. }
        | CostPart::UntapType { .. }
        | CostPart::Unattach { .. }
        | CostPart::FlipCoin(_)
        | CostPart::RollDice { .. } => true,
        CostPart::AddCounter { counter_type, .. } => *counter_type != CounterType::M1M1,
        _ => false,
    }
}

/// Mirrors Java's `CostPart.isRenewable()`.
pub fn is_renewable(part: &CostPart) -> bool {
    matches!(
        part,
        CostPart::Tap
            | CostPart::Untap
            | CostPart::Reveal { .. }
            | CostPart::TapType { .. }
            | CostPart::UntapType { .. }
    )
}

/// Mirrors Java's `CostPart.isUndoable()`.
pub fn is_undoable(part: &CostPart) -> bool {
    matches!(
        part,
        CostPart::Tap | CostPart::Untap | CostPart::ChooseColor(_) | CostPart::Mana { .. }
    )
}

/// Mirrors Java's `CostPart.getTypeDescription()`.
pub fn get_type_description(part: &CostPart) -> Option<&str> {
    match part {
        CostPart::Unattach {
            description: Some(d),
            ..
        } => Some(d.as_str()),
        _ => None,
    }
}

/// Mirrors Java's `CostPart.getDescriptiveType()`.
pub fn get_descriptive_type(part: &CostPart) -> String {
    if let Some(desc) = get_type_description(part) {
        return desc.to_string();
    }

    match part {
        CostPart::Sacrifice { type_filter, .. }
        | CostPart::Discard { type_filter, .. }
        | CostPart::Exile { type_filter, .. }
        | CostPart::Return { type_filter, .. }
        | CostPart::TapType { type_filter, .. }
        | CostPart::UntapType { type_filter, .. }
        | CostPart::GainControl { type_filter, .. }
        | CostPart::RemoveAnyCounter { type_filter, .. }
        | CostPart::ExiledMoveToGrave { type_filter, .. }
        | CostPart::ExileFromStack { type_filter, .. }
        | CostPart::PutCardToLib { type_filter, .. }
        | CostPart::Enlist { type_filter, .. }
        | CostPart::Behold { type_filter, .. }
        | CostPart::ExileCtrlOrGrave { type_filter, .. }
        | CostPart::ExileFromAnyGrave { type_filter, .. }
        | CostPart::ExileFromSameGrave { type_filter, .. }
        | CostPart::Reveal { type_filter, .. }
        | CostPart::Exert { type_filter, .. }
        | CostPart::Unattach { type_filter, .. } => type_filter.to_lowercase(),
        _ => format!("{part:?}").to_lowercase(),
    }
}

pub fn is_core_type(s: &str) -> bool {
    matches!(
        s,
        "Creature"
            | "Artifact"
            | "Enchantment"
            | "Land"
            | "Planeswalker"
            | "Instant"
            | "Sorcery"
            | "Tribal"
            | "Battle"
            | "Kindred"
    )
}

impl PartialEq for CostPart {
    fn eq(&self, other: &Self) -> bool {
        payment_order(self) == payment_order(other)
    }
}

impl Eq for CostPart {}

impl PartialOrd for CostPart {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CostPart {
    fn cmp(&self, other: &Self) -> Ordering {
        payment_order(self).cmp(&payment_order(other))
    }
}
