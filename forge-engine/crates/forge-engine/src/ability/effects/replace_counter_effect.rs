//! ReplaceCounter effect — replace counter placement with another effect.
//!
//! Ported from Java's `ReplaceCounterEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // Counter replacement is handled by the replacement handler system.
}
