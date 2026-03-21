//! ReplaceToken effect — replace token creation with another effect.
//!
//! Ported from Java's `ReplaceTokenEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // Token replacement is handled by the replacement handler system.
}
