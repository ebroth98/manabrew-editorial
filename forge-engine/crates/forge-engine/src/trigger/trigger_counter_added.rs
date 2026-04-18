use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterAdded {
    pub valid_card: Option<String>,
    pub counter_type: Option<String>,
}

impl TriggerCounterAdded {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            counter_type: params.get_cloned(keys::COUNTER_TYPE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterAdded {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterAdded
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
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        let card = sa.trigger_objects.get("Card");
        let player = sa.trigger_objects.get("Player");
        if let Some(c) = card {
            format!("AddedOnce: {}", c)
        } else if let Some(p) = player {
            format!("AddedOnce: {}", p)
        } else {
            String::new()
        }
    }
}
