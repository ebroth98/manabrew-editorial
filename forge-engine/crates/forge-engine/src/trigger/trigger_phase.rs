use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Phase: {}",
        sa.trigger_objects
            .get("Player")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
