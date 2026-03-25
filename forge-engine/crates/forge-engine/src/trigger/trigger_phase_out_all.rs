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
    let TriggerMode::PhaseOutAll { valid_cards } = mode else {
        panic!("Expected PhaseOutAll mode");
    };

    let Some(cards) = params.cards.as_ref() else {
        return valid_cards.is_none();
    };

    cards
        .iter()
        .any(|&cid| check_card_filter(valid_cards, Some(cid), host_card, host_controller, game))
}
