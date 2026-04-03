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
    if let TriggerMode::FightOnce { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            || check_card_filter(valid_card, params.card2, host_card, host_controller, game);
    }
    panic!("Expected FightOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Fighters", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Fighter 1: " + fighters.get(0) + ", Fighter 2: " + fighters.get(1)
    let fighters_csv = sa.get_triggering_object("Fighters").unwrap_or_default();
    let parts: Vec<&str> = fighters_csv.split(',').collect();
    let f1 = parts.first().copied().unwrap_or("");
    let f2 = parts.get(1).copied().unwrap_or("");
    format!("Fighter 1: {}, Fighter 2: {}", f1, f2)
}
