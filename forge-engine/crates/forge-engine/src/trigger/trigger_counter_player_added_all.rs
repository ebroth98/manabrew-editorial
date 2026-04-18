use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{matches_valid_card, matches_valid_player, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCounterPlayerAddedAll {
    pub valid_source: Option<String>,
    pub valid_object: Option<String>,
    pub valid_object_to_source: Option<String>,
}

impl TriggerCounterPlayerAddedAll {
    pub fn parse(
        valid_source: Option<String>,
        valid_object: Option<String>,
        valid_object_to_source: Option<String>,
    ) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source,
            valid_object,
            valid_object_to_source,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCounterPlayerAddedAll {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CounterPlayerAddedAll
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if let Some(filter) = &self.valid_source {
            let source_ok = if let Some(cid) = params.source_card.or(params.card) {
                matches_valid_card(filter, cid, host_card, host_controller, game)
            } else if let Some(pid) = params.source_player {
                matches_valid_player(filter, pid, host_controller)
            } else {
                false
            };
            if !source_ok {
                return false;
            }
        }

        if let Some(filter) = &self.valid_object {
            let object_ok = if let Some(cid) = params.object_card {
                matches_valid_card(filter, cid, host_card, host_controller, game)
            } else if let Some(pid) = params.object_player {
                matches_valid_player(filter, pid, host_controller)
            } else {
                false
            };
            if !object_ok {
                return false;
            }
        }

        if let Some(filter) = &self.valid_object_to_source {
            let Some(source_player) = params.source_player else {
                return false;
            };
            let object_ok = if let Some(pid) = params.object_player {
                matches_valid_player(filter, pid, source_player)
            } else if let Some(cid) = params.object_card {
                let card_controller = game.card(cid).controller;
                if filter.contains("YouCtrl") {
                    card_controller == source_player
                } else if filter.contains("OppCtrl") {
                    card_controller != source_player
                } else {
                    matches_valid_card(filter, cid, host_card, host_controller, game)
                }
            } else {
                false
            };
            if !object_ok {
                return false;
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
        // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Source, AbilityKey.Object, AbilityKey.CounterMap)
        // Java also sets Amount = sum of CounterMap values
        if let Some(source) = params.source_player {
            sa.set_triggering_object("Source", &source.0.to_string());
        } else if let Some(source) = params.source_card {
            sa.set_triggering_object("Source", &source.0.to_string());
        }
        if let Some(obj) = params.object_card {
            sa.set_triggering_object("Object", &obj.0.to_string());
        } else if let Some(p) = params.object_player {
            sa.set_triggering_object("Object", &p.0.to_string());
        }
        // TODO: Java also sets CounterMap from runParams and computes Amount as sum of CounterMap values.
        // CounterMap is a Map<CounterType, Integer> in Java. Using counter_amount as approximation.
        if let Some(amount) = params.counter_amount {
            sa.set_triggering_object("Amount", &amount.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "AddedOnce: {}: {}",
            sa.trigger_objects
                .get("Source")
                .cloned()
                .unwrap_or_default(),
            sa.trigger_objects
                .get("Object")
                .cloned()
                .unwrap_or_default()
        )
    }
}
