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
    let TriggerMode::Mentored {
        valid_card,
        valid_source,
    } = mode
    else {
        panic!("Expected Mentored mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
        && check_card_filter(
            valid_source,
            params.source_card.or(params.card2),
            host_card,
            host_controller,
            game,
        )
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(src) = params.source_card {
        sa.add_triggering_object("Source", &src.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Mentor: {}, Mentored: {}",
        sa.trigger_objects
            .get("Source")
            .map(|s| s.as_str())
            .unwrap_or(""),
        sa.trigger_objects
            .get("Card")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
