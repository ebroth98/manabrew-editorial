use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}",
        sa.get_triggering_object("Player").unwrap_or_default()
    )
}
