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
    let TriggerMode::Investigated {
        valid_player,
        first_time_only,
    } = mode
    else {
        panic!("Expected Investigated mode");
    };

    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if *first_time_only && params.first_time != Some(true) {
        return false;
    }
    true
}
