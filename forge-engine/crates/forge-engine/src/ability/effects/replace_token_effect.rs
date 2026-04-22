//! ReplaceToken effect — replace token creation with another effect.
//!
//! Ported from Java's `ReplaceTokenEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ReplaceTokenEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(ReplaceTokenEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Token replacement is handled by the replacement handler system.
}
