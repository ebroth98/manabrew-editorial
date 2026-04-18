use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerUnlockDoor {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
    pub this_door: bool,
}

impl TriggerUnlockDoor {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_player: params.get_cloned(keys::VALID_PLAYER),
            this_door: params.is_true("ThisDoor"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerUnlockDoor {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::UnlockDoor
    }

    fn perform_test(
        &self,
        trigger: &Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !check_card_filter(&self.valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if !check_player_filter(&self.valid_player, params.player, host_controller) {
            return false;
        }
        if self.this_door && params.card != Some(host_card) {
            return false;
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
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Player: {}, Card: {}",
            sa.trigger_objects
                .get("Player")
                .map(|s| s.as_str())
                .unwrap_or(""),
            sa.trigger_objects
                .get("Card")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
