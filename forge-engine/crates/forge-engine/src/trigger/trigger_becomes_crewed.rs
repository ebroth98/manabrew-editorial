use super::trigger::{check_card_filter, matches_amount, TriggerMode};
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
    let TriggerMode::BecomesCrewed {
        valid_card,
        valid_crew,
        first_time_crewed,
        valid_crew_amount,
    } = mode
    else {
        panic!("Expected BecomesCrewed mode");
    };

    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    let Some(crews) = params.crew_cards.as_ref() else {
        return false;
    };
    if !crews
        .iter()
        .any(|&cid| check_card_filter(valid_crew, Some(cid), host_card, host_controller, game))
    {
        return false;
    }
    if *first_time_crewed && params.first_time != Some(true) {
        return false;
    }
    if let Some(amount_filter) = valid_crew_amount {
        if !matches_amount(amount_filter, crews.len()) {
            return false;
        }
    }
    true
}
