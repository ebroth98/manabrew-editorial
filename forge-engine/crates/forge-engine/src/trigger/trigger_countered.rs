use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, matches_valid_card, matches_valid_sa, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCountered {
    pub valid_card: Option<String>,
    pub valid_cause: Option<String>,
    pub valid_sa: Option<String>,
}

impl TriggerCountered {
    pub fn parse(
        valid_card: Option<String>,
        valid_cause: Option<String>,
        valid_sa: Option<String>,
    ) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card,
            valid_cause,
            valid_sa,
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCountered {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Countered
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
        if let Some(filter) = &self.valid_cause {
            if let Some(cause) = params.cause.as_ref() {
                let Some(cause_card) = cause.source else {
                    return false;
                };
                if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(filter) = &self.valid_sa {
            if let Some(countered_sa) = params.spell_ability.as_ref() {
                if !matches_valid_sa(filter, countered_sa) {
                    return false;
                }
            } else {
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
        // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Card, AbilityKey.Cause, AbilityKey.CounteredSA)
        if let Some(card) = params.card {
            sa.set_triggering_object("Card", &card.0.to_string());
        }
        // TODO: Java also sets Cause (SpellAbility) and CounteredSA (SpellAbility) from runParams.
        // Skipping Cause and CounteredSA for now since SpellAbility is complex and stored as object in Java.
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: "Countered: " + Card + ", Cause: " + Cause
        format!(
            "Countered: {}, Cause: {}",
            sa.get_triggering_object("Card").unwrap_or(""),
            sa.get_triggering_object("Cause").unwrap_or("")
        )
    }
}
