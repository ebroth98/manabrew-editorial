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
    let TriggerMode::PayEcho { valid_card, paid } = mode else {
        panic!("Expected PayEcho mode");
    };
    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if let Some(expected) = paid {
        return params.echo_paid == Some(*expected);
    }
    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
}

pub fn get_important_stack_objects(_sa: &SpellAbility) -> String {
    String::new()
}
