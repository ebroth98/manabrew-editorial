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
    let TriggerMode::CrewedSaddled {
        valid_card,
        valid_crew,
    } = mode
    else {
        panic!("Expected CrewedSaddled mode");
    };

    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    let Some(crews) = params.crew_cards.as_ref() else {
        return false;
    };
    crews
        .iter()
        .any(|&cid| check_card_filter(valid_crew, Some(cid), host_card, host_controller, game))
}
