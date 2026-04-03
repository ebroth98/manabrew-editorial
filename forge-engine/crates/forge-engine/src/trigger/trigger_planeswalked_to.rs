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
    let TriggerMode::PlaneswalkedTo { valid_card } = mode else {
        panic!("Expected PlaneswalkedTo mode");
    };

    let Some(cards) = params.cards.as_ref() else {
        return valid_card.is_none();
    };

    cards
        .iter()
        .any(|&cid| check_card_filter(valid_card, Some(cid), host_card, host_controller, game))
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Cards", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "PlaneswalkedTo: {}",
        sa.trigger_objects
            .get("Cards")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
