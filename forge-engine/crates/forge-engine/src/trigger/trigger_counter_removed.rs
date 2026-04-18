use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterRemoved {
    pub valid_card: Option<String>,
    pub counter_type: Option<String>,
}

impl TriggerCounterRemoved {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            counter_type: params.get_cloned(keys::COUNTER_TYPE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterRemoved {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterRemoved
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
            && check_counter_type_filter(&self.counter_type, &params.counter_type)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "RemovedFrom: {}",
            sa.trigger_objects.get("Card").cloned().unwrap_or_default()
        )
    }
}
