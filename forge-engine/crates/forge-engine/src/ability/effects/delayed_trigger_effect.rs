use super::EffectContext;
use crate::spellability::SpellAbility;

/// End-of-turn / next-turn registration callback. Mirrors the `GameCommand.run()`
/// lambdas in Java `DelayedTriggerEffect` that register a delayed trigger for
/// the next turn or upcoming turn when the appropriate phase boundary is reached.
///
/// Re-invokes the delayed trigger registration on the trigger handler so that
/// the trigger becomes active during the next turn cycle.
pub fn run(ctx: &mut EffectContext, sa: &SpellAbility) {
    crate::trigger::resolve_delayed_trigger(ctx, sa);
}

/// Mirrors Java's `DelayedTriggerEffect` (core path).
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    crate::trigger::resolve_delayed_trigger(ctx, sa);
}
