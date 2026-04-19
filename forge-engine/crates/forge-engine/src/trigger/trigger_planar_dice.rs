use serde::{Deserialize, Serialize};

use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::TriggerBehavior;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPlanarDice {
    pub valid_player: Option<String>,
    pub result: Option<String>,
}

impl TriggerPlanarDice {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_player: params.get_cloned(keys::VALID_PLAYER),
            result: params.get_cloned("Result"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerPlanarDice {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::PlanarDice
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        _game: &GameState,
    ) -> bool {
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !trigger.matches_optional_valid_player_filter(&self.valid_player, params.player) {
            return false;
        }
        if let Some(expected) = self.result.as_ref() {
            return params.mode.as_ref() == Some(expected);
        }
        true
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(p) = params.player {
            sa.set_triggering_object(crate::ability::AbilityKey::Player, &p.0.to_string());
        }
    }

    fn get_important_stack_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &SpellAbility,
    ) -> String {
        format!(
            "Roller: {}",
            sa.trigger_objects
                .get(&crate::ability::AbilityKey::Player)
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
