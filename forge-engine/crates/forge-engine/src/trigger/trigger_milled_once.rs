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
    let TriggerMode::MilledOnce {
        valid_card,
        valid_player,
    } = mode
    else {
        panic!("Expected MilledOnce mode");
    };
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    let Some(cards) = params.cards.as_ref() else {
        return false;
    };
    cards
        .iter()
        .any(|&cid| check_card_filter(valid_card, Some(cid), host_card, host_controller, game))
}
