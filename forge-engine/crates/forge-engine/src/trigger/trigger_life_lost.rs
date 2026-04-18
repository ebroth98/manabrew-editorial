use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_player_filter, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerLifeLost {
    pub valid_player: Option<String>,
    pub first_time_only: bool,
}

impl TriggerLifeLost {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_player: params.get_cloned(keys::VALID_PLAYER),
            first_time_only: params.has("FirstTime"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerLifeLost {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::LifeLost
    }

    fn perform_test(
        &self,
        trigger: &Trigger,
        params: &RunParams,
        _game: &GameState,
    ) -> bool {
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if !check_player_filter(&self.valid_player, params.player, host_controller) {
            return false;
        }
        if self.first_time_only && params.first_time != Some(true) {
            return false;
        }
        true
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(amount) = params.life_amount {
            sa.set_triggering_object("LifeAmount", &amount.to_string());
        }
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        // Java: "Player: " + Player + ", LostAmount: " + LifeAmount
        format!(
            "Player: {}, LostAmount: {}",
            sa.get_triggering_object("Player").unwrap_or_default(),
            sa.get_triggering_object("LifeAmount").unwrap_or_default()
        )
    }
}
