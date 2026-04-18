use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterAddedOnce {
    pub valid_card: Option<String>,
    pub counter_type: Option<String>,
    pub valid_source: Option<String>,
}

impl TriggerCounterAddedOnce {
    pub fn parse(
        valid_card: Option<String>,
        counter_type: Option<String>,
        valid_source: Option<String>,
    ) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card,
            counter_type,
            valid_source,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterAddedOnce {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterAddedOnce
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !check_card_filter(&self.valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if !check_counter_type_filter(&self.counter_type, &params.counter_type) {
            return false;
        }
        if let Some(filter) = &self.valid_source {
            if filter.eq_ignore_ascii_case("You") {
                return params.cause_player == Some(host_controller);
            }
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
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
        if let Some(amount) = params.counter_amount {
            sa.set_triggering_object("Amount", &amount.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        let target = sa
            .trigger_objects
            .get("Card")
            .or(sa.trigger_objects.get("Player"));
        format!(
            "AddedOnce: {}, Amount: {}",
            target.cloned().unwrap_or_default(),
            sa.trigger_objects
                .get("Amount")
                .cloned()
                .unwrap_or_default()
        )
    }
}
