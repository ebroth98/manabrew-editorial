use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAbilityActivated {
    pub valid_card: Option<String>,
    pub valid_activating_player: Option<String>,
}

impl TriggerAbilityActivated {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_activating_player: params.get_cloned(keys::VALID_ACTIVATING_PLAYER),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAbilityActivated {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::AbilityActivated
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(&self.valid_card, params.card, host_card, host_controller, game)
            && check_player_filter(&self.valid_activating_player, params.player, host_controller)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        _sa: &mut SpellAbility,
        _params: &RunParams,
        _game: &GameState,
    ) {
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, _sa: &SpellAbility) -> String {
        String::new()
    }
}
