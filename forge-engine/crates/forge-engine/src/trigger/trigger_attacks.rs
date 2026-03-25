use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let alone = params.is_true(keys::ALONE);
    TriggerMode::Attacks { valid_card, alone }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Attacks { valid_card, alone } = mode {
        if *alone && params.num_attackers.unwrap_or(0) != 1 {
            return false;
        }
        return check_card_filter(
            valid_card,
            params.attacker,
            host_card,
            host_controller,
            game,
        );
    }
    panic!("Expected Attacks mode");
}
