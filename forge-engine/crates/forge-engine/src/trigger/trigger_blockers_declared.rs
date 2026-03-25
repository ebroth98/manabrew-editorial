use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::Params,
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
    if let TriggerMode::BlockersDeclared = mode {
        return true;
    }
    panic!("Expected BlockersDeclared mode");
}

pub fn parse_mode(_params: &Params) -> TriggerMode {
    TriggerMode::BlockersDeclared
}
