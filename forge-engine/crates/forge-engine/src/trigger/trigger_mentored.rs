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
    let TriggerMode::Mentored {
        valid_card,
        valid_source,
    } = mode
    else {
        panic!("Expected Mentored mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
        && check_card_filter(
            valid_source,
            params.source_card.or(params.card2),
            host_card,
            host_controller,
            game,
        )
}
