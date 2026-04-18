use serde::{Deserialize, Serialize};

use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::{keys, Params},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAttackerBlockedByCreature {
    pub valid_card: Option<String>,
    pub valid_blocked: Option<String>,
}

impl TriggerAttackerBlockedByCreature {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_blocked: params.get_cloned(keys::VALID_BLOCKED),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAttackerBlockedByCreature {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::AttackerBlockedByCreature
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(&self.valid_card, params.blocker, host_card, host_controller, game)
            && check_card_filter(
                &self.valid_blocked,
                params.blocked_attacker,
                host_card,
                host_controller,
                game,
            )
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(attacker) = params.attacker {
            sa.set_triggering_object("Attacker", &attacker.0.to_string());
        }
        if let Some(blocker) = params.blocker {
            sa.set_triggering_object("Blocker", &blocker.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Attacker: {}, Blocker: {}",
            sa.get_triggering_object("Attacker").unwrap_or(""),
            sa.get_triggering_object("Blocker").unwrap_or("")
        )
    }
}
