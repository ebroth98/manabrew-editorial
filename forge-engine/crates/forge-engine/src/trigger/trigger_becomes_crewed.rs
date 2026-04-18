use serde::{Deserialize, Serialize};

use super::trigger::{check_card_filter, matches_amount, TriggerBehavior};
use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::Params,
    spellability::SpellAbility,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBecomesCrewed {
    pub valid_card: Option<String>,
    pub valid_crew: Option<String>,
    pub first_time_crewed: bool,
    pub valid_crew_amount: Option<String>,
}

impl TriggerBecomesCrewed {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned("ValidCard"),
            valid_crew: params.get_cloned("ValidCrew"),
            first_time_crewed: params.has("FirstTimeCrewed"),
            valid_crew_amount: params.get_cloned("ValidCrewAmount"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerBecomesCrewed {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::BecomesCrewed
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
        let Some(crews) = params.crew_cards.as_ref() else {
            return false;
        };
        if !crews
            .iter()
            .any(|&cid| check_card_filter(&self.valid_crew, Some(cid), host_card, host_controller, game))
        {
            return false;
        }
        if self.first_time_crewed && params.first_time != Some(true) {
            return false;
        }
        if let Some(amount_filter) = &self.valid_crew_amount {
            if !matches_amount(amount_filter, crews.len()) {
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
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
        if let Some(crew) = params.crew_cards.as_ref() {
            let csv = crew
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object("Crew", &csv);
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Vehicle: {}  Crew: {}",
            sa.get_triggering_object("Card").unwrap_or(""),
            sa.get_triggering_object("Crew").unwrap_or("")
        )
    }
}
