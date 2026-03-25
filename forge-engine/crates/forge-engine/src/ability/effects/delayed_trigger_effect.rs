use super::EffectContext;
use crate::spellability::SpellAbility;

/// Mirrors Java's `DelayedTriggerEffect` (core path).
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    crate::trigger::resolve_delayed_trigger(ctx, sa);
}
