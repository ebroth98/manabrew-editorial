use serde::{Deserialize, Serialize};

use super::trigger::{check_card_filter, TriggerBehavior};
use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::{keys, Params},
    spellability::SpellAbility,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAbandoned {
    pub valid_card: Option<String>,
}

impl TriggerAbandoned {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAbandoned {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Abandoned
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
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(v) = params.card.as_ref() {
            sa.set_triggering_object("Scheme", &v.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, _sa: &SpellAbility) -> String {
        String::new()
    }
}
