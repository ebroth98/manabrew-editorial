use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Phase {
        phase,
        valid_player,
    } = mode
    {
        if let Some(expected_phase) = phase {
            if params.phase != Some(*expected_phase) {
                return false;
            }
        }
        return check_player_filter(valid_player, params.player, host_controller);
    }
    panic!("Expected Phase mode");
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let phase = params
        .get(keys::PHASE)
        .and_then(super::trigger::parse_phase);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    TriggerMode::Phase {
        phase,
        valid_player,
    }
}
