//! CountersRemoveAll effect — remove all counters of a type from permanents.
//!
//! Ported from Java's `CountersRemoveAllEffect.java`.

use super::{parse_counter_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use forge_foundation::ZoneType;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `CountersRemoveAllEffect` class extending `SpellAbilityEffect`.
pub struct CountersRemoveAllEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for CountersRemoveAllEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let counter_type_str = sa
        .params
        .get(keys::COUNTER_TYPE)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "P1P1".to_string());
    let counter_type = parse_counter_type(&counter_type_str);

    let targets: Vec<crate::ids::CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card.into_iter().collect()
    } else {
        // Remove from all permanents matching ValidCard$
        let valid = sa.valid_card().unwrap_or("Permanent");
        ctx.game
            .cards
            .iter()
            .filter(|c| {
                c.zone == ZoneType::Battlefield && super::matches_change_type(c, valid, &[])
            })
            .map(|c| c.id)
            .collect()
    };

    for card_id in targets {
        let current = *ctx
            .game
            .card(card_id)
            .counters
            .get(&counter_type)
            .unwrap_or(&0);
        if current > 0 {
            ctx.game
                .card_mut(card_id)
                .remove_counter(&counter_type, current);
        }
    }
    }
}
