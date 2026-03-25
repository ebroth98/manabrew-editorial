use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::keys,
    parsing::Params,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::Abandoned { valid_card } = mode else {
        panic!("Expected Abandoned mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    TriggerMode::Abandoned { valid_card }
}
