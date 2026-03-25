use super::trigger::TriggerMode;
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

pub fn perform_test(
    mode: &TriggerMode,
    _params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    _host_controller: PlayerId,
) -> bool {
    let TriggerMode::DayTimeChanges = mode else {
        panic!("Expected DayTimeChanges mode");
    };
    true
}
