use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{
    check_counter_type_filter, matches_valid_card, matches_valid_player, TriggerBehavior,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterAddedAll {
    pub counter_type: Option<String>,
    pub valid: Option<String>,
}

impl TriggerCounterAddedAll {
    pub fn parse(counter_type: Option<String>, valid: Option<String>) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            counter_type,
            valid,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterAddedAll {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterAddedAll
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !check_counter_type_filter(&self.counter_type, &params.counter_type) {
            return false;
        }

        let Some(valid_filter) = self.valid.as_deref() else {
            return true;
        };

        if let Some(cid) = params.object_card.or(params.card) {
            return matches_valid_card(valid_filter, cid, host_card, host_controller, game);
        }
        if let Some(pid) = params.object_player.or(params.player) {
            return matches_valid_player(valid_filter, pid, host_controller);
        }
        false
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(cards) = params.cards.as_ref() {
            let csv = cards
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object("Objects", &csv);
        }
        if let Some(amount) = params.counter_amount {
            sa.set_triggering_object("Amount", &amount.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Amount: {}",
            sa.trigger_objects
                .get("Amount")
                .cloned()
                .unwrap_or_default()
        )
    }
}
