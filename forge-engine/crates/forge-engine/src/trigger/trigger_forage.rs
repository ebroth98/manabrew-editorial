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
    if let TriggerMode::Forage { valid_player } = mode {
        return check_player_filter(valid_player, params.player, host_controller);
    }
    panic!("Expected Forage mode");
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
