use super::trigger::TriggerMode;
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    _params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    _host_controller: PlayerId,
) -> bool {
    let TriggerMode::DayTimeChanges = mode else {
        panic!("Expected DayTimeChanges mode");
    };
    true
}

pub fn set_triggering_objects(_sa: &mut SpellAbility, _params: &RunParams) {}

pub fn get_important_stack_objects(_sa: &SpellAbility) -> String {
    String::new()
}
