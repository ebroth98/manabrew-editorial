use super::trigger::{check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::Investigated {
        valid_player,
        first_time_only,
    } = mode
    else {
        panic!("Expected Investigated mode");
    };

    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if *first_time_only && params.first_time != Some(true) {
        return false;
    }
    true
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
