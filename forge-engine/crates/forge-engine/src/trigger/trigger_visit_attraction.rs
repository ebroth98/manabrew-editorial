use super::trigger::{check_card_filter, check_player_filter, TriggerMode};
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
    let TriggerMode::VisitAttraction {
        valid_player,
        valid_card,
    } = mode
    else {
        panic!("Expected VisitAttraction mode");
    };
    check_player_filter(valid_player, params.player, host_controller)
        && check_card_filter(valid_card, params.card, host_card, host_controller, game)
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}",
        sa.trigger_objects
            .get("Player")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
