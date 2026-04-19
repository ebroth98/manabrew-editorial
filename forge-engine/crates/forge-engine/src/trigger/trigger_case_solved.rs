use serde::{Deserialize, Serialize};

use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::TriggerBehavior;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCaseSolved {
    pub valid_card: Option<String>,
    pub valid_player: Option<String>,
}

impl TriggerCaseSolved {
    pub fn parse(
        valid_card: Option<String>,
        valid_player: Option<String>,
    ) -> Box<dyn TriggerBehavior> {
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
        trigger.matches_optional_valid_card_filter(&self.valid_card, params.card, game)
            && trigger.matches_optional_valid_player_filter(&self.valid_player, params.player)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(card) = params.card {
            sa.set_triggering_object(crate::ability::AbilityKey::Card, &card.0.to_string());
        }
        if let Some(p) = params.player {
            sa.set_triggering_object(crate::ability::AbilityKey::Player, &p.0.to_string());
        }
    }

    fn get_important_stack_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &SpellAbility,
    ) -> String {
        format!(
            "Card: {}",
            sa.get_triggering_object(crate::ability::AbilityKey::Card)
                .unwrap_or_default()
        )
    }
}
