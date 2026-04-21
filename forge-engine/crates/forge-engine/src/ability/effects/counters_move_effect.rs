//! CountersMove effect — move counters between permanents.
//!
//! Ported from Java's `CountersMoveEffect.java`.
//! Move N counters of a type from one permanent to another.

use super::{parse_counter_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use forge_foundation::ZoneType;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `CountersMoveEffect` class extending `SpellAbilityEffect`.
pub struct CountersMoveEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for CountersMoveEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let counter_type_str = sa
        .params
        .get(keys::COUNTER_TYPE)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "P1P1".to_string());
    let counter_type = parse_counter_type(&counter_type_str);
    let amount = super::resolve_numeric_svar(ctx.game, sa, "CounterNum", 1).max(0);

    // Source: card to remove counters from
    let source_card = sa
        .source
        .filter(|&cid| ctx.game.card(cid).zone == ZoneType::Battlefield);
    // Target: card to add counters to
    let target_card = sa
        .target_chosen
        .target_card
        .filter(|&cid| ctx.game.card(cid).zone == ZoneType::Battlefield);

    let (Some(from), Some(to)) = (source_card, target_card) else {
        return;
    };
    if from == to {
        return;
    }

    // Remove counters from source
    let current = *ctx
        .game
        .card(from)
        .counters
        .get(&counter_type)
        .unwrap_or(&0);
    let to_move = amount.min(current);
    if to_move <= 0 {
        return;
    }

    ctx.game
        .card_mut(from)
        .remove_counter(&counter_type, to_move);
    ctx.game.card_mut(to).add_counter(&counter_type, to_move);
    }
}
