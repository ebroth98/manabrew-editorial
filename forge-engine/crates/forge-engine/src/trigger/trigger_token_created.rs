use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerTokenCreated {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
}

impl TriggerTokenCreated {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        let valid_card = params
            .get("ValidToken")
            .or_else(|| params.get(keys::VALID_CARD))
            .map(|s| s.to_string());
        let valid_player = params.get_cloned(keys::VALID_PLAYER);
        Box::new(Self {
            valid_card,
            valid_player,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerTokenCreated {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::TokenCreated
    }

    fn perform_test(&self, trigger: &Trigger, params: &RunParams, game: &GameState) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !trigger.matches_optional_valid_player_filter(&self.valid_player, params.player) {
            return false;
        }
        trigger.matches_optional_valid_card_filter(&self.valid_card, params.card, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(p) = params.player {
            sa.set_triggering_object(crate::ability::AbilityKey::Player, &p.0.to_string());
        }
        if let Some(card) = params.card {
            sa.set_triggering_object(crate::ability::AbilityKey::Card, &card.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Player: {}",
            sa.trigger_objects
                .get(&crate::ability::AbilityKey::Player)
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
