use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDestroyed {
    pub valid_card: Option<String>,
    pub valid_causer: Option<String>,
}

impl TriggerDestroyed {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_causer: params.get_cloned("ValidCauser"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerDestroyed {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Destroyed
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
            && check_card_filter(
                &self.valid_causer,
                params
                    .causer
                    .or(params.cause_card)
                    .or_else(|| params.cause.as_ref().and_then(|sa| sa.source)),
                host_card,
                host_controller,
                game,
            )
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Card, AbilityKey.Causer)
        if let Some(card_id) = params.card {
            sa.set_triggering_object("Card", &card_id.0.to_string());
        }
        if let Some(causer) = params.causer {
            sa.set_triggering_object("Causer", &causer.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: "Destroyed: " + Card + ", Destroyer: " + Causer
        format!(
            "Destroyed: {}, Destroyer: {}",
            sa.get_triggering_object("Card").unwrap_or(""),
            sa.get_triggering_object("Causer").unwrap_or("")
        )
    }
}
