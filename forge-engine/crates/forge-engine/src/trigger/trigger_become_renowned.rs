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
    let TriggerMode::BecomeRenowned { valid_card } = mode else {
        panic!("Expected BecomeRenowned mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Renowned: {}",
        sa.get_triggering_object("Card").unwrap_or("")
    )
}
