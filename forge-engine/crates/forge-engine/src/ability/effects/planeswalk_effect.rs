//! Planeswalk — move to a new plane (Planechase).
//! Ported from Java's PlaneswalkEffect: leaves current plane, moves to new one.
//! Planechase format support — fires trigger for plane change.

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Run Planeswalk replacement effects before planeswalking.
    let mut event = ReplacementEvent::Planeswalk {
        player: sa.activating_player,
    };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
        return;
    }

    // Fire Planeswalk trigger
    ctx.trigger_handler.run_trigger(
        TriggerType::Planeswalk,
        RunParams {
            player: Some(sa.activating_player),
            ..Default::default()
        },
        false,
    );
}
