use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_player_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_cause = params.get_cloned(keys::VALID_CAUSE);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    let attacker = params
        .get(keys::ATTACKER)
        .map(|v| v.eq_ignore_ascii_case("true"));
    let require_first_time = params.has("FirstTime");
    TriggerMode::Taps {
        valid_card,
        valid_cause,
        valid_player,
        attacker,
        require_first_time,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Taps {
        valid_card,
        valid_cause,
        valid_player,
        attacker,
        require_first_time,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = valid_cause {
            let Some(cause_sa) = params.cause.as_ref() else {
                return false;
            };
            let Some(cause_card) = cause_sa.source else {
                return false;
            };
            if !super::trigger::matches_valid_card(
                filter,
                cause_card,
                host_card,
                host_controller,
                game,
            ) {
                return false;
            }
        }
        if !check_player_filter(valid_player, params.player, host_controller) {
            return false;
        }
        if let Some(expected_attacker) = attacker {
            if params.attacker.is_some() != *expected_attacker {
                return false;
            }
        }
        if *require_first_time && params.first_time != Some(true) {
            return false;
        }
        return true;
    }
    panic!("Expected Taps mode");
}
