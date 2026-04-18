use serde::{Deserialize, Serialize};

use crate::card::valid_filter;
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::TriggerBehavior;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBecomesTarget {
    pub valid_source: Option<String>,
    pub valid_target: Option<String>,
    pub require_first_time: bool,
    pub require_valiant: bool,
}

impl TriggerBecomesTarget {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source: params.get_cloned(keys::VALID_SOURCE),
            valid_target: params.get_cloned(keys::VALID_TARGET),
            require_first_time: params.has("FirstTime"),
            require_valiant: params.has("Valiant"),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerBecomesTarget {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::BecomesTarget
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if let Some(filter) = self.valid_source.as_ref() {
            let source_matches = if let Some(source_sa) = params.source_sa.as_ref() {
                if filter.starts_with("SpellAbility") {
                    let parts: Vec<&str> = filter.split('.').collect();
                    let kind_matches = parts
                        .first()
                        .is_none_or(|part| part.eq_ignore_ascii_case("SpellAbility"));
                    if !kind_matches {
                        false
                    } else {
                        parts.iter().skip(1).all(|part| match *part {
                            "OppCtrl" => source_sa.activating_player != host_controller,
                            "YouCtrl" => source_sa.activating_player == host_controller,
                            _ => true,
                        })
                    }
                } else {
                    source_sa.source.is_some_and(|source_card| {
                        super::trigger::matches_valid_card(
                            filter,
                            source_card,
                            host_card,
                            host_controller,
                            game,
                        )
                    })
                }
            } else if let Some(source_card) = params.cause_card {
                super::trigger::matches_valid_card(
                    filter,
                    source_card,
                    host_card,
                    host_controller,
                    game,
                )
            } else {
                false
            };
            if !source_matches {
                return false;
            }
        }

        if let Some(filter) = self.valid_target.as_ref() {
            let target_card = params.target_card.or(params.card);
            let target_player = params.target_player.or(params.player);
            let host = game.card(host_card);
            if !valid_filter::matches_valid(
                filter,
                target_card.map(|id| game.card(id)),
                target_player,
                host,
                host_controller,
            ) {
                return false;
            }
        }

        if self.require_first_time && params.first_time != Some(true) {
            return false;
        }
        if self.require_valiant && params.valiant != Some(true) {
            return false;
        }
        true
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(ref source_sa) = params.source_sa {
            if let Some(source_card) = source_sa.source {
                sa.set_triggering_object("Source", &source_card.0.to_string());
            }
            sa.set_triggering_spell_ability("SourceSA", source_sa.clone());
        }
        if let Some(card) = params.target_card.or(params.card) {
            sa.set_triggering_object("Target", &card.0.to_string());
        } else if let Some(p) = params.target_player {
            sa.set_triggering_object("Target", &p.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        format!(
            "Source: {}, Target: {}",
            sa.get_triggering_object("Source").unwrap_or(""),
            sa.get_triggering_object("Target").unwrap_or("")
        )
    }
}
