use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_damage_target, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExcessDamage {
    pub valid_source: Option<String>,
    pub valid_target: Option<String>,
}

impl TriggerExcessDamage {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source: params.get_cloned(keys::VALID_SOURCE),
            valid_target: params.get_cloned(keys::VALID_TARGET),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerExcessDamage {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::ExcessDamage
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
            &self.valid_source,
            params.damage_source,
            host_card,
            host_controller,
            game,
        ) && check_damage_target(
            &self.valid_target,
            params,
            host_card,
            host_controller,
            game,
            false,
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
        if let Some(amount) = params.damage_amount {
            sa.set_triggering_object("DamageAmount", &amount.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: "Damaged: " + Target + ", Amount: " + DamageAmount
        format!(
            "Damaged: {}, Amount: {}",
            sa.get_triggering_object("Target").unwrap_or_default(),
            sa.get_triggering_object("DamageAmount").unwrap_or_default()
        )
    }
}
