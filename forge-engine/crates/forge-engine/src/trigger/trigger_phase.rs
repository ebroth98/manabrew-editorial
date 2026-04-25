use serde::{Deserialize, Serialize};

use crate::event::RunParams;
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;
use crate::trigger::TriggerType;

use super::trigger::TriggerBehavior;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPhase {
    pub phase: Option<forge_foundation::PhaseType>,
    pub valid_player: Option<crate::parsing::CompiledSelector>,
}

impl TriggerPhase {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        let phase = params
            .get(keys::PHASE)
            .and_then(forge_foundation::PhaseType::from_script_name);
        let valid_player = params.selector_cloned(keys::VALID_PLAYER);
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
        game: &GameState,
    ) -> bool {
        let host_controller = trigger.base.card_trait_base.host_controller(game);
        if let Some(expected_phase) = self.phase {
            if params.phase != Some(expected_phase) {
                return false;
            }
        }
        trigger.matches_optional_valid_player_filter(&self.valid_player, params.player, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        game: &GameState,
    ) {
        if let Some(p) = params.player {
            sa.set_triggering_object(crate::ability::AbilityKey::Player, &p.0.to_string());
            sa.set_triggering_object(crate::ability::AbilityKey::TriggeredPlayer, p);
        }
    }

    fn get_important_stack_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &SpellAbility,
    ) -> String {
        format!(
            "Phase: {}",
            sa.trigger_objects
                .get(&crate::ability::AbilityKey::Player)
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
