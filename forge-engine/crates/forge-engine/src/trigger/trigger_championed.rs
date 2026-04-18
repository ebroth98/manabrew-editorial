use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerChampioned {
    pub valid_card: Option<String>,
    pub valid_source: Option<String>,
}

impl TriggerChampioned {
    pub fn parse(valid_card: Option<String>, valid_source: Option<String>) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card,
            valid_source,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerChampioned {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Championed
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(
            &self.valid_card,
            params.championed_card.or(params.card),
            host_card,
            host_controller,
            game,
        ) && check_card_filter(&self.valid_source, params.card, host_card, host_controller, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(c) = params.championed_card {
            sa.set_triggering_object("Championed", &c.0.to_string());
        }
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Championed: {}",
            sa.trigger_objects
                .get("Championed")
                .cloned()
                .unwrap_or_default()
        )
    }
}
