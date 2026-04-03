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
    let TriggerMode::Championed {
        valid_card,
        valid_source,
    } = mode
    else {
        panic!("Expected Championed mode");
    };
    check_card_filter(
        valid_card,
        params.championed_card.or(params.card),
        host_card,
        host_controller,
        game,
    ) && check_card_filter(valid_source, params.card, host_card, host_controller, game)
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(c) = params.championed_card {
        sa.add_triggering_object("Championed", &c.0.to_string());
    }
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Championed: {}",
        sa.trigger_objects
            .get("Championed")
            .cloned()
            .unwrap_or_default()
    )
}
