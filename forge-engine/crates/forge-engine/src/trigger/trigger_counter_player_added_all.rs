use super::trigger::{matches_valid_card, matches_valid_player, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
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
