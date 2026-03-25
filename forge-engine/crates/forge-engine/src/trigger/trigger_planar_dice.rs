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
    let TriggerMode::PlanarDice {
        valid_player,
        result,
    } = mode
    else {
        panic!("Expected PlanarDice mode");
    };

    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if let Some(expected) = result {
        return params.mode.as_ref() == Some(expected);
    }
    true
}
