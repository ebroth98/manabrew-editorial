use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::FlippedCoin {
        valid_player,
        valid_result,
    } = mode
    {
        if !check_player_filter(valid_player, params.player, host_controller) {
            return false;
        }
        if let Some(filter) = valid_result {
            let Some(won) = params.coin_flip_won else {
                return false;
            };
            let f = filter.trim();
            if (f.eq_ignore_ascii_case("Win") || f.eq_ignore_ascii_case("Heads")) && !won {
                return false;
            }
            if (f.eq_ignore_ascii_case("Lose") || f.eq_ignore_ascii_case("Tails")) && won {
                return false;
            }
        }
        return true;
    }
    panic!("Expected FlippedCoin mode");
}
