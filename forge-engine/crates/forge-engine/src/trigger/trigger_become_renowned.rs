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
    let TriggerMode::BecomeRenowned { valid_card } = mode else {
        panic!("Expected BecomeRenowned mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
}
