use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::ClassLevelGained {
        valid_card,
        class_level,
    } = mode
    else {
        panic!("Expected ClassLevelGained mode");
    };
    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if let Some(expected) = class_level {
        return params.class_level == Some(*expected);
    }
    true
}
