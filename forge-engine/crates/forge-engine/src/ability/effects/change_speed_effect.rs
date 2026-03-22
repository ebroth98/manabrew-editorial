//! ChangeSpeed — change a permanent's speed (digital-only, Alchemy).
//! Ported from Java's ChangeSpeedEffect.
//! In our engine this is a no-op since speed is an Arena-specific concept.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // Digital-only speed mechanic from Arena/Alchemy. No game state to modify.
}
