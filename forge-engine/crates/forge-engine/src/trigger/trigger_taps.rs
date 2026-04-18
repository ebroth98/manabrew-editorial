use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerTaps {
    pub valid_card: Option<String>,
    pub valid_cause: Option<String>,
    pub valid_player: Option<String>,
    pub attacker: Option<bool>,
    pub require_first_time: bool,
}

impl TriggerTaps {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_cause: params.get_cloned(keys::VALID_CAUSE),
            valid_player: params.get_cloned(keys::VALID_PLAYER),
            attacker: params
                .get(keys::ATTACKER)
                .map(|v| v.eq_ignore_ascii_case("true")),
            require_first_time: params.has("FirstTime"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerTaps {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Taps
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
        if let Some(filter) = self.valid_cause.as_ref() {
            let Some(cause_sa) = params.cause.as_ref() else {
                return false;
            };
            let Some(cause_card) = cause_sa.source else {
                return false;
            };
            if !super::trigger::matches_valid_card(
                filter,
                cause_card,
                host_card,
                host_controller,
                game,
            ) {
                return false;
            }
        }
        if !check_player_filter(&self.valid_player, params.player, host_controller) {
            return false;
        }
        if let Some(expected_attacker) = self.attacker {
            if params.attacker.is_some() != expected_attacker {
                return false;
            }
        }
        if self.require_first_time && params.first_time != Some(true) {
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
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Tapped: {}",
            sa.trigger_objects
                .get("Card")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
