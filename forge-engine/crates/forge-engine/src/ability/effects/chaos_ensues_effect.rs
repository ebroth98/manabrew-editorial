//! ChaosEnsues — trigger chaos ability in Planechase.
//! Ported from Java's ChaosEnsuesEffect: fires the ChaosEnsues trigger
//! which causes all Planechase chaos abilities to trigger.

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ChaosEnsuesEffect` class extending `SpellAbilityEffect`.
pub struct ChaosEnsuesEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for ChaosEnsuesEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
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
}
