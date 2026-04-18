use serde::{Deserialize, Serialize};

use crate::parsing::{keys, Params};
use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAttacks {
    pub valid_card: Option<String>,
    pub alone: bool,
}

impl TriggerAttacks {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            alone: params.is_true(keys::ALONE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAttacks {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Attacks
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if self.alone && params.num_attackers.unwrap_or(0) != 1 {
            return false;
        }
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
        // Java: sa.setTriggeringObject(AbilityKey.Defender, runParams.get(AbilityKey.Attacked));
        if let Some(p) = params.attacked_player {
            sa.set_triggering_object("Defender", &p.0.to_string());
        } else if let Some(c) = params.attacked_card {
            sa.set_triggering_object("Defender", &c.0.to_string());
        }
        // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Attacker, AbilityKey.Defenders, AbilityKey.DefendingPlayer);
        if let Some(attacker) = params.attacker {
            sa.set_triggering_object("Attacker", &attacker.0.to_string());
        }
        // Defenders combines both player and card defender IDs
        {
            let mut parts = Vec::new();
            if let Some(players) = params.defenders_player_ids.as_ref() {
                for p in players {
                    parts.push(p.0.to_string());
                }
            }
            if let Some(cards) = params.defenders_card_ids.as_ref() {
                for c in cards {
                    parts.push(c.0.to_string());
                }
            }
            if !parts.is_empty() {
                sa.set_triggering_object("Defenders", &parts.join(","));
            }
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
