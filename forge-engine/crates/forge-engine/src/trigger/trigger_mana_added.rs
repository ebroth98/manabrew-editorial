use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, matches_valid_sa, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerManaAdded {
    pub valid_source: Option<String>,
    pub valid_sa: Option<String>,
    pub player: Option<String>,
    pub produced: Option<String>,
}

impl TriggerManaAdded {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source: params.get_cloned("ValidSource"),
            valid_sa: params.get_cloned(keys::VALID_SA),
            player: params.get_cloned(keys::PLAYER),
            produced: params.get_cloned(keys::PRODUCED),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerManaAdded {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::ManaAdded
    }

    fn perform_test(
        &self,
        trigger: &Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !check_card_filter(&self.valid_source, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = self.valid_sa.as_ref() {
            let Some(sa) = params.ability_mana.as_ref() else {
                return false;
            };
            if !matches_valid_sa(filter, sa) {
                return false;
            }
        }
        if !check_player_filter(&self.player, params.player, host_controller) {
            return false;
        }
        if let Some(expected) = self.produced.as_ref() {
            let Some(actual) = params.produced.as_ref() else {
                return false;
            };
            if !actual.contains(expected) {
                return false;
            }
        }
        true
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
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
        if let Some(produced) = params.produced.as_ref() {
            sa.set_triggering_object("Produced", produced);
        }
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Produced: {}",
            sa.trigger_objects
                .get("Produced")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
