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
    let TriggerMode::ConjureAll {
        valid_player,
        valid_card,
    } = mode
    else {
        panic!("Expected ConjureAll mode");
    };
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    let Some(cards) = params.cards.as_ref() else {
        return valid_card.is_none();
    };
    cards
        .iter()
        .any(|&cid| check_card_filter(valid_card, Some(cid), host_card, host_controller, game))
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // TODO: Java filters cards by ValidCard param before setting.
    // We don't have access to trigger params here, passing through all cards.
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Cards", &csv);
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    // TODO: Java also sets Cause from runParams via
    // sa.setTriggeringObjectsFrom(runParams, AbilityKey.Cause)
    // Skipping Cause for now since SpellAbility is complex and stored as object in Java
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}",
        sa.trigger_objects
            .get("Player")
            .cloned()
            .unwrap_or_default()
    )
}
