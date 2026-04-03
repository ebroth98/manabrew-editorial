use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::DiscardedAll {
        valid_card,
        valid_player,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_player_filter(valid_player, params.player, host_controller);
    }
    panic!("Expected DiscardedAll mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: filters cards with ValidCard via CardLists.getValidCards, then sets Cards, Amount, Player, Cause
    // TODO: ValidCard filtering skipped — free function has no access to trigger params (hasParam/getParam)
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
    // TODO: AbilityKey.Cause is a SpellAbility in Java, cannot be stored as String easily
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}, Amount: {}",
        sa.get_triggering_object("Player").unwrap_or_default(),
        sa.get_triggering_object("Amount").unwrap_or_default()
    )
}
