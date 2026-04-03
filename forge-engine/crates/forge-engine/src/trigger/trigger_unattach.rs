use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Unattached { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected Unattached mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(obj) = params.object_card {
        sa.add_triggering_object("Object", &obj.0.to_string());
    }
    if let Some(src) = params.source_card {
        sa.add_triggering_object("AttachSource", &src.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Object: {}, Attachment: {}",
        sa.trigger_objects
            .get("Object")
            .map(|s| s.as_str())
            .unwrap_or(""),
        sa.trigger_objects
            .get("AttachSource")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
