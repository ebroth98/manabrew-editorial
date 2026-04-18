use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_damage_target, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDamageDoneOnceByController {
    pub valid_source: Option<String>,
    pub valid_target: Option<String>,
    pub combat_damage_only: bool,
}

impl TriggerDamageDoneOnceByController {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source: params.get_cloned(keys::VALID_SOURCE),
            valid_target: params.get_cloned(keys::VALID_TARGET),
            combat_damage_only: params.is_true(keys::COMBAT_DAMAGE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerDamageDoneOnceByController {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::DamageDoneOnceByController
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if self.combat_damage_only && params.is_combat_damage != Some(true) {
            return false;
        }
        if !check_damage_target(&self.valid_target, params, host_card, host_controller, game, true) {
            return false;
        }
        check_card_filter(
            &self.valid_source,
            params.damage_source,
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
        if let Some(card) = params.damage_target_card {
            sa.set_triggering_object("Target", &card.0.to_string());
        } else if let Some(player) = params.damage_target_player {
            sa.set_triggering_object("Target", &player.0.to_string());
        }
        if let Some(src) = params.damage_source {
            sa.set_triggering_object("Source", &src.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: if Target != null { "Damaged: " + Target + ", " } + "Damage Source: " + Source
        let target = sa.get_triggering_object("Target").unwrap_or("");
        if target.is_empty() {
            format!(
                "Damage Source: {}",
                sa.get_triggering_object("Source").unwrap_or("")
            )
        } else {
            format!(
                "Damaged: {}, Damage Source: {}",
                target,
                sa.get_triggering_object("Source").unwrap_or("")
            )
        }
    }
}
