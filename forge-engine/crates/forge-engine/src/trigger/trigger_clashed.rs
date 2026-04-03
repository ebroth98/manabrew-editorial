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
    let TriggerMode::Clashed { valid_player, won } = mode else {
        panic!("Expected Clashed mode");
    };
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if let Some(expected) = won {
        return params.clash_won == Some(*expected);
    }
    true
}

pub fn set_triggering_objects(_sa: &mut SpellAbility, _params: &RunParams) {
    // Clash has no triggered variables
}

pub fn get_important_stack_objects(_sa: &SpellAbility) -> String {
    String::new()
}
