use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPayEcho {
    pub valid_card: Option<String>,
    pub paid: Option<bool>,
}

impl TriggerPayEcho {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            paid: params.get("Paid").map(|v| v.eq_ignore_ascii_case("true")),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerPayEcho {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::PayEcho
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
        if let Some(expected) = self.paid {
            return params.echo_paid == Some(expected);
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
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, _sa: &SpellAbility) -> String {
        String::new()
    }
}
