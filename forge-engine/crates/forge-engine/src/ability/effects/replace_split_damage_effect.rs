//! ReplaceSplitDamage effect — replace split damage assignment.
//!
//! Ported from Java's `ReplaceSplitDamageEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ReplaceSplitDamageEffect` class extending `SpellAbilityEffect`.
pub struct ReplaceSplitDamageEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for ReplaceSplitDamageEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Split damage replacement is handled by the replacement handler system.
    }
}
