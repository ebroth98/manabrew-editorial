use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, matches_valid_card, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDiscarded {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
    pub valid_cause: Option<String>,
}

impl TriggerDiscarded {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_player: params.get_cloned(keys::VALID_PLAYER),
            valid_cause: params.get_cloned(keys::VALID_CAUSE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerDiscarded {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Discarded
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
        if !check_player_filter(&self.valid_player, params.player, host_controller) {
            return false;
        }
        if let Some(filter) = self.valid_cause.as_ref() {
            let Some(cause_sa) = params.cause.as_ref() else {
                return false;
            };
            let Some(cause_card) = cause_sa.source else {
                return false;
            };
            if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
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
        // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Card, AbilityKey.Cause);
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
        // TODO: AbilityKey.Cause is a SpellAbility in Java, cannot be stored as String easily
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: "Discarded: " + Card + ", Cause: " + Cause
        format!(
            "Discarded: {}, Cause: {}",
            sa.get_triggering_object("Card").unwrap_or_default(),
            sa.get_triggering_object("Cause").unwrap_or_default()
        )
    }
}
