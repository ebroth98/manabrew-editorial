//! ReplaceSplitDamage effect — replace split damage assignment.
//!
//! Ported from Java's `ReplaceSplitDamageEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // Split damage replacement is handled by the replacement handler system.
}
