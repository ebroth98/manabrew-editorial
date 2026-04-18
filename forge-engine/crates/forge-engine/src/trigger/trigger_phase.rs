use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_player_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPhase {
    pub phase: Option<forge_foundation::PhaseType>,
    pub valid_player: Option<String>,
}

impl TriggerPhase {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        let phase = params
            .get(keys::PHASE)
            .and_then(super::trigger::parse_phase);
        let valid_player = params.get_cloned(keys::VALID_PLAYER);
        Box::new(Self {
            phase,
            valid_player,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerPhase {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Phase
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        _game: &GameState,
    ) -> bool {
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if let Some(expected_phase) = self.phase {
            if params.phase != Some(expected_phase) {
                return false;
            }
        }
        check_player_filter(&self.valid_player, params.player, host_controller)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
            sa.set_triggering_object("TriggeredPlayer", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Phase: {}",
            sa.trigger_objects
                .get("Player")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
