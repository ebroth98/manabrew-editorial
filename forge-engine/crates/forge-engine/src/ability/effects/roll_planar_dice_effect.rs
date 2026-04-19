//! RollPlanarDice — roll the planar die (Planechase).
//! Ported from Java's RollPlanarDiceEffect: rolls the planar die which
//! can result in Planeswalk, Chaos, or blank.

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Run RollPlanarDice replacement effects before rolling.
    let mut event = ReplacementEvent::RollPlanarDice {
        player: sa.activating_player,
    };
    let repl_result = apply_replacements(ctx.game, &mut event);
    if repl_result == ReplacementResult::Skipped || repl_result == ReplacementResult::Replaced {
        return;
    }

    // Roll 1-6: 1 = Planeswalk, 2 = Chaos, 3-6 = Blank
    let result = ctx.rng.next_int(6) + 1;

    match result {
        1 => {
            // Planeswalk trigger
            ctx.trigger_handler.run_trigger(
                TriggerType::Planeswalk,
                RunParams {
                    player: Some(sa.activating_player),
                    ..Default::default()
                },
                false,
            );
        }
        2 => {
            // Chaos trigger
            ctx.trigger_handler.run_trigger(
                TriggerType::ChaosEnsues,
                RunParams {
                    player: Some(sa.activating_player),
                    ..Default::default()
                },
                false,
            );
        }
        _ => {
            // Blank — nothing happens
        }
    }
}
