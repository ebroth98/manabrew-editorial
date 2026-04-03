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
    let TriggerMode::ClassLevelGained {
        valid_card,
        class_level,
    } = mode
    else {
        panic!("Expected ClassLevelGained mode");
    };
    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if let Some(expected) = class_level {
        return params.class_level == Some(*expected);
    }
    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(level) = params.class_level {
        sa.add_triggering_object("ClassLevel", &level.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Class Level: {}",
        sa.trigger_objects
            .get("ClassLevel")
            .cloned()
            .unwrap_or_default()
    )
}
