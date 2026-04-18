use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerUnattach {
    pub valid_card: Option<String>,
}

impl TriggerUnattach {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerUnattach {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Unattached
    }

    fn perform_test(
        &self,
        trigger: &Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(&self.valid_card, params.card, host_card, host_controller, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(obj) = params.object_card {
            sa.set_triggering_object("Object", &obj.0.to_string());
        }
        if let Some(src) = params.source_card {
            sa.set_triggering_object("AttachSource", &src.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Object: {}, Attachment: {}",
            sa.trigger_objects
                .get("Object")
                .map(|s| s.as_str())
                .unwrap_or(""),
            sa.trigger_objects
                .get("AttachSource")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
