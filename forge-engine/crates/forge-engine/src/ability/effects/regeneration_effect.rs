//! Regeneration — set up a regeneration shield (older mechanic).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `RegenerationEffect` class extending `SpellAbilityEffect`.
pub struct RegenerationEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for RegenerationEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    if let Some(target) = sa.target_chosen.target_card.or(sa.source) {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).regeneration_shields += 1;
        }
    }
    }
}
