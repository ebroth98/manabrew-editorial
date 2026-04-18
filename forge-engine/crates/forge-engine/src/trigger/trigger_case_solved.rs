use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_player_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCaseSolved {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
}

impl TriggerCaseSolved {
    pub fn parse(valid_card: Option<String>, valid_player: Option<String>) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card,
            valid_player,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCaseSolved {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CaseSolved
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
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Card: {}",
            sa.trigger_objects.get("Card").cloned().unwrap_or_default()
        )
    }
}
