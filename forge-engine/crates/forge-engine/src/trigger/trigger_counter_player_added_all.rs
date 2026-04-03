use super::trigger::{matches_valid_card, matches_valid_player, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::CounterPlayerAddedAll {
        valid_source,
        valid_object,
        valid_object_to_source,
    } = mode
    else {
        panic!("Expected CounterPlayerAddedAll mode");
    };

    if let Some(filter) = valid_source {
        let source_ok = if let Some(cid) = params.source_card.or(params.card) {
            matches_valid_card(filter, cid, host_card, host_controller, game)
        } else if let Some(pid) = params.source_player {
            matches_valid_player(filter, pid, host_controller)
        } else {
            false
        };
        if !source_ok {
            return false;
        }
    }

    if let Some(filter) = valid_object {
        let object_ok = if let Some(cid) = params.object_card {
            matches_valid_card(filter, cid, host_card, host_controller, game)
        } else if let Some(pid) = params.object_player {
            matches_valid_player(filter, pid, host_controller)
        } else {
            false
        };
        if !object_ok {
            return false;
        }
    }

    if let Some(filter) = valid_object_to_source {
        let Some(source_player) = params.source_player else {
            return false;
        };
        let object_ok = if let Some(pid) = params.object_player {
            matches_valid_player(filter, pid, source_player)
        } else if let Some(cid) = params.object_card {
            let card_controller = game.card(cid).controller;
            if filter.contains("YouCtrl") {
                card_controller == source_player
            } else if filter.contains("OppCtrl") {
                card_controller != source_player
            } else {
                matches_valid_card(filter, cid, host_card, host_controller, game)
            }
        } else {
            false
        };
        if !object_ok {
            return false;
        }
    }

    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Source, AbilityKey.Object, AbilityKey.CounterMap)
    // Java also sets Amount = sum of CounterMap values
    if let Some(source) = params.source_player {
        sa.add_triggering_object("Source", &source.0.to_string());
    } else if let Some(source) = params.source_card {
        sa.add_triggering_object("Source", &source.0.to_string());
    }
    if let Some(obj) = params.object_card {
        sa.add_triggering_object("Object", &obj.0.to_string());
    } else if let Some(p) = params.object_player {
        sa.add_triggering_object("Object", &p.0.to_string());
    }
    // TODO: Java also sets CounterMap from runParams and computes Amount as sum of CounterMap values.
    // CounterMap is a Map<CounterType, Integer> in Java. Using counter_amount as approximation.
    if let Some(amount) = params.counter_amount {
        sa.add_triggering_object("Amount", &amount.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "AddedOnce: {}: {}",
        sa.trigger_objects
            .get("Source")
            .cloned()
            .unwrap_or_default(),
        sa.trigger_objects
            .get("Object")
            .cloned()
            .unwrap_or_default()
    )
}
