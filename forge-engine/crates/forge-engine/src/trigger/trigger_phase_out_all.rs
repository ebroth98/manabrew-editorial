use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::TriggerBehavior;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPhaseOutAll {
    pub valid_cards: Option<String>,
}

impl TriggerPhaseOutAll {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_cards: params
                .get(keys::VALID_CARDS)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string()),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerPhaseOutAll {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::PhaseOutAll
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        let Some(cards) = params.cards.as_ref() else {
            return self.valid_cards.is_none();
        };

        cards.iter().any(|&cid| {
            trigger.matches_optional_valid_card_filter(&self.valid_cards, Some(cid), game)
        })
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        // TODO: port ValidCards filtering from Java (IterableUtil.filter with CardPredicates.restriction)
        if let Some(cards) = params.cards.as_ref() {
            let csv = cards
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object(crate::ability::AbilityKey::Cards, &csv);
        }
    }

    fn get_important_stack_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &SpellAbility,
    ) -> String {
        format!(
            "PhasedOut: {}",
            sa.trigger_objects
                .get(&crate::ability::AbilityKey::Cards)
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
