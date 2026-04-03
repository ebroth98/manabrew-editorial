use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::TriggerMode;

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let _ = (params, game, host_card, host_controller);
    if let TriggerMode::Always = mode {
        return true;
    }
    panic!("Expected Always mode");
}

pub fn set_triggering_objects(_sa: &mut SpellAbility, _params: &RunParams) {
}

pub fn get_important_stack_objects(_sa: &SpellAbility) -> String {
    String::new()
}
