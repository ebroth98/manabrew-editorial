use super::trigger::{check_card_filter, check_player_filter, TriggerMode};
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
    let TriggerMode::CrankContraption {
        valid_card,
        valid_player,
    } = mode
    else {
        panic!("Expected CrankContraption mode");
    };

    check_card_filter(valid_card, params.card, host_card, host_controller, game)
        && check_player_filter(valid_player, params.player, host_controller)
}
