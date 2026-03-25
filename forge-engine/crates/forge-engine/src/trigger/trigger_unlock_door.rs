use super::trigger::{check_card_filter, check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::UnlockDoor {
        valid_card,
        valid_player,
        this_door,
    } = mode
    else {
        panic!("Expected UnlockDoor mode");
    };
    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if *this_door && params.card != Some(host_card) {
        return false;
    }
    true
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    let this_door = params.is_true("ThisDoor");
    TriggerMode::UnlockDoor {
        valid_card,
        valid_player,
        this_door,
    }
}
