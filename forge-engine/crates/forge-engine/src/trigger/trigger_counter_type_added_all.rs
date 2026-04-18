use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{matches_valid_card, matches_valid_player, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterTypeAddedAll {
    pub valid_object: Option<String>,
    pub first_time_only: bool,
}

impl TriggerCounterTypeAddedAll {
    pub fn parse(valid_object: Option<String>, first_time_only: bool) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_object,
            first_time_only,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterTypeAddedAll {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterTypeAddedAll
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if let Some(filter) = &self.valid_object {
            let object_ok = if let Some(cid) = params.object_card.or(params.card) {
                matches_valid_card(filter, cid, host_card, host_controller, game)
            } else if let Some(pid) = params.object_player.or(params.player) {
                matches_valid_player(filter, pid, host_controller)
            } else {
                false
            };
            if !object_ok {
                return false;
            }
        }

        if self.first_time_only && params.first_time != Some(true) {
            return false;
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
        if let Some(obj) = params.object_card {
            sa.set_triggering_object("Object", &obj.0.to_string());
        } else if let Some(p) = params.object_player {
            sa.set_triggering_object("Object", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "AddedOnce: {}",
            sa.trigger_objects
                .get("Object")
                .cloned()
                .unwrap_or_default()
        )
    }
}
