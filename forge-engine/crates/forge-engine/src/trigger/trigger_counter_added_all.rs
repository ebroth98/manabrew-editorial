use super::trigger::{
    check_counter_type_filter, matches_valid_card, matches_valid_player, TriggerMode,
};
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
    let TriggerMode::CounterAddedAll {
        counter_type,
        valid,
    } = mode
    else {
        panic!("Expected CounterAddedAll mode");
    };

    if !check_counter_type_filter(counter_type, &params.counter_type) {
        return false;
    }

    let Some(valid_filter) = valid.as_deref() else {
        return true;
    };

    if let Some(cid) = params.object_card.or(params.card) {
        return matches_valid_card(valid_filter, cid, host_card, host_controller, game);
    }
    if let Some(pid) = params.object_player.or(params.player) {
        return matches_valid_player(valid_filter, pid, host_controller);
    }
    false
}
