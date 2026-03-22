//! CleanUpEffect — cleanup/reset effect.
//!
//! Mirrors Java's `CleanUpEffect.java`.
//! Clears remembered cards, chosen cards, chosen players, etc.
//! This is a synonym file — the actual implementation lives in `cleanup_effect.rs`.
//! Java names it `CleanUpEffect` while the Rust convention uses `cleanup_effect`.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve by delegating to the existing `cleanup_effect` handler.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::cleanup_effect::resolve(ctx, sa);
}
