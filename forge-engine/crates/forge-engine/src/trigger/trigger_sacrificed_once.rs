use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSacrificedOnce {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
}

impl TriggerSacrificedOnce {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_player: params.get_cloned(keys::VALID_PLAYER),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerSacrificedOnce {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::SacrificedOnce
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
            && check_player_filter(&self.valid_player, params.player, host_controller)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        // TODO: port ValidCard filtering from Java (CardLists.getValidCards)
        if let Some(cards) = params.cards.as_ref() {
            let csv = cards
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object("Cards", &csv);
            sa.set_triggering_object("Amount", &cards.len().to_string());
        }
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
        // TODO: port SpellAbility triggering object (AbilityKey.Cause)
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Player: {}, Amount: {}",
            sa.trigger_objects
                .get("Player")
                .map(|s| s.as_str())
                .unwrap_or(""),
            sa.trigger_objects
                .get("Amount")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
