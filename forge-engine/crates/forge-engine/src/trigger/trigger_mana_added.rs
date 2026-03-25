use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_player_filter, matches_valid_sa, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::ManaAdded {
        valid_source,
        valid_sa,
        player,
        produced,
    } = mode
    {
        if !check_card_filter(valid_source, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = valid_sa {
            let Some(sa) = params.ability_mana.as_ref() else {
                return false;
            };
            if !matches_valid_sa(filter, sa) {
                return false;
            }
        }
        if !check_player_filter(player, params.player, host_controller) {
            return false;
        }
        if let Some(expected) = produced {
            let Some(actual) = params.produced.as_ref() else {
                return false;
            };
            if !actual.contains(expected) {
                return false;
            }
        }
        return true;
    }
    panic!("Expected ManaAdded mode");
}
