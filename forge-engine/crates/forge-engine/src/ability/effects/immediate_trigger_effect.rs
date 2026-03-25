//! ImmediateTrigger effect — fire a trigger immediately without waiting.
//!
//! Ported from Java's `ImmediateTriggerEffect.java`.
//! Creates and immediately fires a trigger, bypassing the normal trigger queue.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    crate::trigger::resolve_immediate_trigger(ctx, sa);
}
