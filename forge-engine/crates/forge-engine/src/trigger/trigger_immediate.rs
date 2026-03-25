use crate::ability::effects::EffectContext;
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
