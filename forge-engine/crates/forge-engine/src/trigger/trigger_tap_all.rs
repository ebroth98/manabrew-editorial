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
    if let TriggerMode::TapAll { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected TapAll mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // TODO: port ValidCards filtering from Java (IterableUtil.filter with CardPredicates.restriction)
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
        "Tapped: {}",
        sa.trigger_objects
            .get("Cards")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
