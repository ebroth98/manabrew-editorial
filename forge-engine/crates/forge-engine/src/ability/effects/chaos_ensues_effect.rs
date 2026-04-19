//! ChaosEnsues — trigger chaos ability in Planechase.
//! Ported from Java's ChaosEnsuesEffect: fires the ChaosEnsues trigger
//! which causes all Planechase chaos abilities to trigger.

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Fire the ChaosEnsues trigger — Planechase chaos abilities listen for this
    ctx.trigger_handler.run_trigger(
        TriggerType::ChaosEnsues,
        RunParams {
            player: Some(sa.activating_player),
            ..Default::default()
        },
        false,
    );
}
