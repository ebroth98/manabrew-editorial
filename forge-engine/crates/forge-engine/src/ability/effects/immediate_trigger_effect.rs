//! ImmediateTrigger effect — fire a trigger immediately without waiting.
//!
//! Ported from Java's `ImmediateTriggerEffect.java`.
//! Creates and immediately fires a trigger, bypassing the normal trigger queue.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // ImmediateTrigger resolves the trigger's sub-ability directly.
    // The trigger details are in the SA's params and the sub-ability chain
    // handles the actual effect. This is a passthrough that ensures the
    // trigger fires in the correct context.

    // RememberObjects$ — remember defined objects for the trigger
    if let Some(remember_def) = sa.params.get("RememberObjects") {
        if let Some(source_id) = sa.source {
            if remember_def.eq_ignore_ascii_case("Targeted") {
                if let Some(target) = sa.target_chosen.target_card {
                    ctx.game.card_mut(source_id).add_remembered_card(target);
                }
            }
        }
    }

    // The sub-ability chain (SubAbility$) is resolved by the spell resolution
    // pipeline after this effect returns. No additional action needed here.
}
