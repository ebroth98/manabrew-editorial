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
    let TriggerMode::MilledOnce {
        valid_card,
        valid_player,
    } = mode
    else {
        panic!("Expected MilledOnce mode");
    };
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    let Some(cards) = params.cards.as_ref() else {
        return false;
    };
    cards
        .iter()
        .any(|&cid| check_card_filter(valid_card, Some(cid), host_card, host_controller, game))
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // TODO: port ValidCard filtering from Java (CardLists.getValidCards)
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Cards", &csv);
        sa.add_triggering_object("Amount", &cards.len().to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
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
