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
    let TriggerMode::PlaneswalkedTo { valid_card } = mode else {
        panic!("Expected PlaneswalkedTo mode");
    };

    let Some(cards) = params.cards.as_ref() else {
        return valid_card.is_none();
    };

    cards
        .iter()
        .any(|&cid| check_card_filter(valid_card, Some(cid), host_card, host_controller, game))
}
