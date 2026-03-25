use super::trigger::TriggerMode;
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::Params,
};

pub fn perform_test(
    mode: &TriggerMode,
    _params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    _host_controller: PlayerId,
) -> bool {
    let TriggerMode::NewGame = mode else {
        panic!("Expected NewGame mode");
    };
    true
}

pub fn parse_mode(_params: &Params) -> TriggerMode {
    TriggerMode::NewGame
}
