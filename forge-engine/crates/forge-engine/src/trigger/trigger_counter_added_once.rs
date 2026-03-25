use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::CounterAddedOnce {
        valid_card,
        counter_type,
        valid_source,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if !check_counter_type_filter(counter_type, &params.counter_type) {
            return false;
        }
        if let Some(filter) = valid_source {
            if filter.eq_ignore_ascii_case("You") {
                return params.cause_player == Some(host_controller);
            }
        }
        return true;
    }
    panic!("Expected CounterAddedOnce mode");
}
