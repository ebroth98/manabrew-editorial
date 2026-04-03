use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::EnteredRoom {
        valid_card,
        valid_room,
    } = mode
    else {
        panic!("Expected EnteredRoom mode");
    };
    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if let Some(filter) = valid_room {
        let Some(room) = params.room_name.as_ref() else {
            return false;
        };
        return filter
            .split(',')
            .map(|s| s.trim())
            .any(|t| t.eq_ignore_ascii_case(room));
    }
    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(name) = params.room_name.as_ref() {
        sa.add_triggering_object("RoomName", name);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    sa.get_triggering_object("RoomName")
        .map(|r| format!("Room: {}", r))
        .unwrap_or_default()
}
