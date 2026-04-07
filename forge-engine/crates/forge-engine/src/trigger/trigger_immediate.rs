use crate::ability::effects::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::{DelayedTrigger, TriggerMode};

/// Trigger-module owned implementation of ImmediateTrigger resolution.
/// Mirrors Java's `ImmediateTriggerEffect.resolve()` which registers
/// a delayed trigger that fires as soon as possible through normal
/// trigger processing.
pub fn resolve_immediate_trigger(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(remember_def) = sa.params.get(keys::REMEMBER_OBJECTS) {
        if let Some(source_id) = sa.source {
            if remember_def.eq_ignore_ascii_case("Targeted") {
                if let Some(target) = sa.target_chosen.target_card {
                    ctx.game.card_mut(source_id).add_remembered_card(target);
                }
            }
        }
    }

    // Execute$ — register a delayed trigger that fires immediately through
    // normal trigger processing, matching Java's registerDelayedTrigger flow.
    if let Some(execute_name) = sa.params.get(keys::EXECUTE) {
        if let Some(source_id) = sa.source {
            let svar_text = ctx.game.card(source_id).svars.get(execute_name).cloned();
            if svar_text.is_some() {
                let delayed = DelayedTrigger {
                    mode: TriggerType::Immediate,
                    trigger_mode: TriggerMode::Immediate,
                    execute_svar: execute_name.to_string(),
                    controller: sa.activating_player,
                    source_card: source_id,
                    target_card: None,
                    remembered_amount: 0,
                };
                ctx.trigger_handler.register_delayed_trigger(delayed);
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
