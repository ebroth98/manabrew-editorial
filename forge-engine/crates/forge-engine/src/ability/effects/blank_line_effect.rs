//! BlankLine — no-op formatting effect used in card scripts.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `BlankLineEffect` class extending `SpellAbilityEffect`.
pub struct BlankLineEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for BlankLineEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {

    }
}
