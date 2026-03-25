use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Enlisted {
        valid_card,
        valid_enlisted,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_card_filter(
                valid_enlisted,
                params.enlisted,
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected Enlisted mode");
}
