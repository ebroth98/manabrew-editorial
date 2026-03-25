use super::trigger::{check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::Clashed { valid_player, won } = mode else {
        panic!("Expected Clashed mode");
    };
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if let Some(expected) = won {
        return params.clash_won == Some(*expected);
    }
    true
}
