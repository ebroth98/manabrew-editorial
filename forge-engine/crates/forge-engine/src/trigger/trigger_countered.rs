use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, matches_valid_card, matches_valid_sa, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Countered {
        valid_card,
        valid_cause,
        valid_sa,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = valid_cause {
            if let Some(cause) = params.cause.as_ref() {
                let Some(cause_card) = cause.source else {
                    return false;
                };
                if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(filter) = valid_sa {
            if let Some(countered_sa) = params.spell_ability.as_ref() {
                if !matches_valid_sa(filter, countered_sa) {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }
    panic!("Expected Countered mode");
}
