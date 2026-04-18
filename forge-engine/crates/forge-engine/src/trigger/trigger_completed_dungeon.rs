use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::spellability::SpellAbility;

use super::trigger::{check_player_filter, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCompletedDungeon {
    pub valid_player: Option<String>,
}

impl TriggerCompletedDungeon {
    pub fn parse(valid_player: Option<String>) -> Box<dyn TriggerBehavior> {
        Box::new(Self { valid_player })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerCompletedDungeon {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::CompletedDungeon
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        _game: &GameState,
    ) -> bool {
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_player_filter(&self.valid_player, params.player, host_controller)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(p) = params.player {
            sa.set_triggering_object("Player", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Player: {}",
            sa.trigger_objects
                .get("Player")
                .cloned()
                .unwrap_or_default()
        )
    }
}
