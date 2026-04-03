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
    if let TriggerMode::UntapAll { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected UntapAll mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // TODO: port Map<Player, CardCollection> iteration from Java (AbilityKey.Map + filteredMap)
    // Java sets: Map, Player (map.keySet()), Cards (all values combined), Amount (cards.size())
    // For now we pass through cards directly without Map-based filtering
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
        "Amount: {}",
        sa.trigger_objects
            .get("Amount")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
