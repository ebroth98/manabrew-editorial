use serde::{Deserialize, Serialize};

use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::{keys, Params},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAttackerUnblocked {
    pub valid_card: Option<String>,
}

impl TriggerAttackerUnblocked {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAttackerUnblocked {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::AttackerUnblocked
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(
            &self.valid_card,
            params.attacker,
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
        if let Some(c) = params.attacked_card {
            sa.set_triggering_object("Defender", &c.0.to_string());
        } else if let Some(p) = params.attacked_player {
            sa.set_triggering_object("Defender", &p.0.to_string());
        }
        if let Some(p) = params.defending_player {
            sa.set_triggering_object("DefendingPlayer", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Attacker: {}",
            sa.get_triggering_object("Attacker").unwrap_or("")
        )
    }
}
