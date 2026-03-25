use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::TriggerMode;

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let _ = (params, game, host_card, host_controller);
    if let TriggerMode::Always = mode {
        return true;
    }
    panic!("Expected Always mode");
}
