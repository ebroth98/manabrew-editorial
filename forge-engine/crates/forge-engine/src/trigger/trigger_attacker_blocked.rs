use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::AttackerBlocked { valid_card } = mode {
        return check_card_filter(
            valid_card,
            params.attacker,
            host_card,
            host_controller,
            game,
        );
    }
    panic!("Expected AttackerBlocked mode");
}
