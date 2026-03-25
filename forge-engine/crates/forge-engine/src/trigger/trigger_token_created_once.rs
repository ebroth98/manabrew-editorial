use crate::ability::AbilityKey;
use crate::{
    event::{AbilityValue, RunParams},
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};
use crate::card::valid_filter::matches_valid_player;

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::TokenCreatedOnce {
        valid_card,
        only_first,
    } = mode
    {
        let Some(AbilityValue::Cards(cards)) = params.get_value(AbilityKey::Cards) else {
            return false;
        };
        let any_match = cards.iter().any(|&card_id| {
            check_card_filter(valid_card, Some(card_id), host_card, host_controller, game)
        });
        if !any_match {
            return false;
        }
        if let Some(filter) = only_first {
            let Some(AbilityValue::Players(players)) = params.get_value(AbilityKey::FirstTime)
            else {
                return false;
            };
            if !players
                .iter()
                .copied()
                .any(|pid| matches_valid_player(filter, pid, host_controller))
            {
                return false;
            }
        }
        return true;
    }
    panic!("Expected TokenCreatedOnce mode");
}
