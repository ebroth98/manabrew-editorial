use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerMilledAll {
    pub valid_card: Option<String>,
}

impl TriggerMilledAll {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerMilledAll {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::MilledAll
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
            return self.valid_card.is_none();
        };
        cards
            .iter()
            .any(|&cid| check_card_filter(&self.valid_card, Some(cid), host_card, host_controller, game))
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
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Amount: {}",
            sa.trigger_objects
                .get("Amount")
                .map(|s| s.as_str())
                .unwrap_or("")
        )
    }
}
