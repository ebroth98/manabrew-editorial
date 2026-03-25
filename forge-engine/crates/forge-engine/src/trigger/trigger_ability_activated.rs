use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_player_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_activating_player = params.get_cloned(keys::VALID_ACTIVATING_PLAYER);
    TriggerMode::AbilityActivated {
        valid_card,
        valid_activating_player,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::AbilityActivated {
        valid_card,
        valid_activating_player,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_player_filter(valid_activating_player, params.player, host_controller);
    }
    panic!("Expected AbilityActivated mode");
}
