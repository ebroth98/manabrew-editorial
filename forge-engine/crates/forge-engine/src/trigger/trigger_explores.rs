use serde::{Deserialize, Serialize};

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, Trigger, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExplores {
    pub valid_card: Option<String>,
    pub valid_explored: Option<String>,
}

impl TriggerExplores {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_card: params.get_cloned(keys::VALID_CARD),
            valid_explored: params.get_cloned("ValidExplored"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerExplores {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::Explored
    }

    fn perform_test(
        &self,
        trigger: &Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        check_card_filter(&self.valid_card, params.card, host_card, host_controller, game)
            && check_card_filter(
                &self.valid_explored,
                params.explored,
                host_card,
                host_controller,
                game,
            )
    }

    fn set_triggering_objects(
        &self,
        _trigger: &Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        // Java: sa.setTriggeringObject(AbilityKey.Explorer, runParams.get(AbilityKey.Card));
        //       if (runParams.containsKey(AbilityKey.Explored)) sa.setTriggeringObjectsFrom(runParams, AbilityKey.Explored);
        if let Some(card_id) = params.card {
            sa.set_triggering_object("Explorer", &card_id.0.to_string());
        }
        if let Some(explored) = params.explored {
            sa.set_triggering_object("Explored", &explored.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &Trigger, sa: &SpellAbility) -> String {
        // Java: "Explorer: " + Explorer + optional ", Explored: " + Explored
        let mut sb = format!(
            "Explorer: {}",
            sa.get_triggering_object("Explorer").unwrap_or_default()
        );
        if let Some(explored) = sa.get_triggering_object("Explored") {
            sb.push_str(&format!(", Explored: {}", explored));
        }
        sb
    }
}
