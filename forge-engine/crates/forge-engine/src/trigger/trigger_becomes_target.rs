use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::TriggerMode;

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_source = params.get_cloned(keys::VALID_SOURCE);
    let valid_target = params.get_cloned(keys::VALID_TARGET);
    let require_first_time = params.has("FirstTime");
    let require_valiant = params.has("Valiant");
    TriggerMode::BecomesTarget {
        valid_source,
        valid_target,
        require_first_time,
        require_valiant,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::BecomesTarget {
        valid_source,
        valid_target,
        require_first_time,
        require_valiant,
    } = mode
    {
        if let Some(filter) = valid_source {
            let source_matches = if let Some(source_sa) = params.source_sa.as_ref() {
                source_sa.source.is_some_and(|source_card| {
                    super::trigger::matches_valid_card(
                        filter,
                        source_card,
                        host_card,
                        host_controller,
                        game,
                    )
                })
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
        if let Some(filter) = valid_target {
            let matches_target = params
                .target_card
                .or(params.card)
                .is_some_and(|target_card| {
                    super::trigger::matches_valid_card(
                        filter,
                        target_card,
                        host_card,
                        host_controller,
                        game,
                    )
                })
                || params
                    .target_player
                    .or(params.player)
                    .is_some_and(|target_player| {
                        super::trigger::matches_valid_player(filter, target_player, host_controller)
                    });
            if !matches_target {
                return false;
            }
        }
        if *require_first_time && params.first_time != Some(true) {
            return false;
        }
        if *require_valiant && params.valiant != Some(true) {
            return false;
        }
        return true;
    }
    panic!("Expected BecomesTarget mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Source, ((SpellAbility) runParams.get(AbilityKey.SourceSA)).getHostCard());
    if let Some(ref source_sa) = params.source_sa {
        if let Some(source_card) = source_sa.source {
            sa.add_triggering_object("Source", &source_card.0.to_string());
        }
    } else if let Some(source) = params.source_card {
        sa.add_triggering_object("Source", &source.0.to_string());
    }
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.SourceSA, AbilityKey.Target);
    // SourceSA is a complex object; we store what we can
    if let Some(card) = params.target_card.or(params.card) {
        sa.add_triggering_object("Target", &card.0.to_string());
    } else if let Some(p) = params.target_player {
        sa.add_triggering_object("Target", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Source: {}, Target: {}",
        sa.get_triggering_object("Source").unwrap_or(""),
        sa.get_triggering_object("Target").unwrap_or("")
    )
}
