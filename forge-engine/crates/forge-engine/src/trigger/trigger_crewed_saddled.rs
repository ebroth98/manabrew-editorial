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
    let TriggerMode::CrewedSaddled {
        valid_card,
        valid_crew,
    } = mode
    else {
        panic!("Expected CrewedSaddled mode");
    };

    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    let Some(crews) = params.crew_cards.as_ref() else {
        return false;
    };
    crews
        .iter()
        .any(|&cid| check_card_filter(valid_crew, Some(cid), host_card, host_controller, game))
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
    // Java uses two spaces between Card and Crew sections
    format!(
        "Card: {}  Crew: {}",
        sa.get_triggering_object("Card").unwrap_or(""),
        sa.get_triggering_object("Crew").unwrap_or("")
    )
}
