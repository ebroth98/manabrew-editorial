use serde::{Deserialize, Serialize};

use super::trigger::{check_card_filter, matches_valid_sa, TriggerBehavior};
use crate::{
    event::{RunParams, TriggerType},
    game::GameState,
    parsing::Params,
    spellability::SpellAbility,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAbilityResolves {
    pub valid_spell_ability: Option<String>,
    pub valid_source: Option<String>,
}

impl TriggerAbilityResolves {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_spell_ability: params.get_cloned("ValidSpellAbility"),
            valid_source: params.get_cloned("ValidSource"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerAbilityResolves {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::AbilityResolves
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        let Some(sa) = params.spell_ability.as_ref() else {
            return false;
        };
        if let Some(filter) = &self.valid_spell_ability {
            if !matches_valid_sa(filter, sa) {
                return false;
            }
        }
        check_card_filter(&self.valid_source, params.card, host_card, host_controller, game)
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(card) = params.card {
            sa.set_triggering_object("Source", &card.0.to_string());
        }
        // SpellAbility is a complex object, skip for now
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "SpellAbility: {}",
            sa.get_triggering_object("SpellAbility").unwrap_or("")
        )
    }
}
