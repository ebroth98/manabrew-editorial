use serde::{Deserialize, Serialize};

use super::trigger::TriggerBehavior;
use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::Params,
    spellability::SpellAbility,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBecomesSaddled {
    pub valid_saddled: Option<String>,
    pub first_time_saddled: bool,
}

impl TriggerBecomesSaddled {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_saddled: params.get_cloned("ValidSaddled"),
            first_time_saddled: params.has("FirstTimeSaddled"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerBecomesSaddled {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::BecomesSaddled
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !trigger.matches_optional_valid_card_filter(&self.valid_saddled, params.card, game) {
            return false;
        }

        if self.first_time_saddled && params.first_time != Some(true) {
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
        if let Some(card) = params.card {
            sa.set_triggering_object(crate::ability::AbilityKey::Card, &card.0.to_string());
        }
        if let Some(crew) = params.crew_cards.as_ref() {
            let csv = crew
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object(crate::ability::AbilityKey::Crew, &csv);
        }
    }

    fn get_important_stack_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &SpellAbility,
    ) -> String {
        format!(
            "Saddled: {}  SaddledBy: {}",
            sa.get_triggering_object(crate::ability::AbilityKey::Card)
                .unwrap_or(""),
            sa.get_triggering_object(crate::ability::AbilityKey::Crew)
                .unwrap_or("")
        )
    }
}
