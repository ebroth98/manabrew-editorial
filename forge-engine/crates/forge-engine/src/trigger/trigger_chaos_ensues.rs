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
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::ChaosEnsues { valid_player } = mode else {
        panic!("Expected ChaosEnsues mode");
    };

    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }

    if let Some(affected) = params.card {
        if affected != host_card {
            return false;
        }
    }

    true
}
