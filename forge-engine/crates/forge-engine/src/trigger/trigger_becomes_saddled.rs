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
    let TriggerMode::BecomesSaddled {
        valid_saddled,
        first_time_saddled,
    } = mode
    else {
        panic!("Expected BecomesSaddled mode");
    };

    if !check_card_filter(valid_saddled, params.card, host_card, host_controller, game) {
        return false;
    }

    if *first_time_saddled && params.first_time != Some(true) {
        return false;
    }

    true
}
