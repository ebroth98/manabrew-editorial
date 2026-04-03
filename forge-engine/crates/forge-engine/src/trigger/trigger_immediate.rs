use crate::ability::effects::EffectContext;
use crate::event::RunParams;
use crate::spellability::SpellAbility;

/// Trigger-module owned implementation of ImmediateTrigger resolution.
/// Mirrors Java's `TriggerImmediate`.
pub fn resolve_immediate_trigger(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(remember_def) = sa.params.get(crate::parsing::keys::REMEMBER_OBJECTS) {
        if let Some(source_id) = sa.source {
            if remember_def.eq_ignore_ascii_case("Targeted") {
                if let Some(target) = sa.target_chosen.target_card {
                    ctx.game.card_mut(source_id).add_remembered_card(target);
                }
            }
        }
    }
}

/// Java TriggerImmediate parity hook.
pub fn perform_test() -> bool {
    true
}

/// Mirrors Java's TriggerImmediate.setTriggeringObjects().
/// Empty implementation — matches Java.
pub fn set_triggering_objects(_sa: &mut SpellAbility, _params: &RunParams) {
    // Empty - matches Java
}

/// Mirrors Java's TriggerImmediate.getImportantStackObjects().
/// Returns empty string — matches Java.
pub fn get_important_stack_objects(_sa: &SpellAbility) -> String {
    String::new()
}
