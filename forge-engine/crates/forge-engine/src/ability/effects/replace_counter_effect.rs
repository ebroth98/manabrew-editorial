//! ReplaceCounter effect — replace counter placement with another effect.
//!
//! Ported from Java's `ReplaceCounterEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ReplaceCounterEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(ReplaceCounterEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Counter replacement is handled by the replacement handler system.
}
