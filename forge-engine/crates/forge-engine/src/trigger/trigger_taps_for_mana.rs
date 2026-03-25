use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let activator = params.get_cloned(keys::VALID_ACTIVATOR);
    let produced = params.get_cloned(keys::PRODUCED);
    TriggerMode::TapsForMana {
        valid_card,
        activator,
        produced,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::TapsForMana {
        valid_card,
        activator,
        produced,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = activator {
            let Some(player) = params.activator.or(params.player) else {
                return false;
            };
            if !super::trigger::matches_valid_player(filter, player, host_controller) {
                return false;
            }
        }
        if let Some(expected) = produced {
            let Some(actual) = params.produced.as_ref() else {
                return false;
            };
            if !actual.contains(expected) {
                return false;
            }
        }
        return true;
    }
    panic!("Expected TapsForMana mode");
}
