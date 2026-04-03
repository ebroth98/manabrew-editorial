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
    if let TriggerMode::Scry { valid_player } = mode {
        return check_player_filter(valid_player, params.player, host_controller);
    }
    panic!("Expected Scry mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    if let Some(n) = params.num {
        sa.add_triggering_object("ScryNum", &n.to_string());
    }
    // TODO: port ScryBottom triggering object (AbilityKey.ScryBottom) - field not yet in RunParams
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Scryer: {}, {}",
        sa.trigger_objects
            .get("Player")
            .map(|s| s.as_str())
            .unwrap_or(""),
        sa.trigger_objects
            .get("ScryNum")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
