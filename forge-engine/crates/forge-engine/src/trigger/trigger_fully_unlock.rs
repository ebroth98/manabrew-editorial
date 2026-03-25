use super::trigger::{check_card_filter, check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::FullyUnlock {
        valid_card,
        valid_player,
    } = mode
    else {
        panic!("Expected FullyUnlock mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
        && check_player_filter(valid_player, params.player, host_controller)
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    TriggerMode::FullyUnlock {
        valid_card,
        valid_player,
    }
}
