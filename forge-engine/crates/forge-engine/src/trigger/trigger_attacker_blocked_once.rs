use serde::{Deserialize, Serialize};

use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::{keys, Params},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAttackerBlockedOnce {
    pub valid_card: Option<String>,
}

impl TriggerAttackerBlockedOnce {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAttackerBlockedOnce {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::AttackerBlockedOnce
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
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(attackers) = params.attacker_ids.as_ref() {
            let csv = attackers
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sa.set_triggering_object("Attackers", &csv);
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Attackers: {}",
            sa.get_triggering_object("Attackers").unwrap_or("")
        )
    }
}
