use serde::{Deserialize, Serialize};

use crate::event::RunParams;
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;
use crate::trigger::TriggerType;

use super::trigger::{Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExerted {
    pub valid_card: Option<crate::parsing::CompiledSelector>,
}

impl TriggerExerted {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.selector_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerExerted {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Exerted
    }

    fn perform_test(&self, trigger: &Trigger, params: &RunParams, game: &GameState) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        trigger.matches_optional_valid_card_filter(&self.valid_card, params.card, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
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

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        format!(
            "Exerted: {}",
            sa.get_triggering_object(crate::ability::AbilityKey::Card)
                .unwrap_or_default()
        )
    }
}
