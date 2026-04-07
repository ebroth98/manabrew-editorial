use super::trigger::{check_card_filter, matches_amount, TriggerMode};
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
    let TriggerMode::BecomesCrewed {
        valid_card,
        valid_crew,
        first_time_crewed,
        valid_crew_amount,
    } = mode
    else {
        panic!("Expected BecomesCrewed mode");
    };

    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    let Some(crews) = params.crew_cards.as_ref() else {
        return false;
    };
    if !crews
        .iter()
        .any(|&cid| check_card_filter(valid_crew, Some(cid), host_card, host_controller, game))
    {
        return false;
    }
    if *first_time_crewed && params.first_time != Some(true) {
        return false;
    }
    if let Some(amount_filter) = valid_crew_amount {
        if !matches_amount(amount_filter, crews.len()) {
            return false;
        }
    }
    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(crew) = params.crew_cards.as_ref() {
        let csv = crew
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Crew", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Vehicle: {}  Crew: {}",
        sa.get_triggering_object("Card").unwrap_or(""),
        sa.get_triggering_object("Crew").unwrap_or("")
    )
}
