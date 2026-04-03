use super::trigger::{check_card_filter, check_player_filter, matches_valid_card, TriggerMode};
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
    let TriggerMode::Discarded {
        valid_card,
        valid_player,
        valid_cause,
    } = mode
    else {
        panic!("Expected Discarded mode");
    };

    if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
        return false;
    }
    if !check_player_filter(valid_player, params.player, host_controller) {
        return false;
    }
    if let Some(filter) = valid_cause {
        let Some(cause_sa) = params.cause.as_ref() else {
            return false;
        };
        let Some(cause_card) = cause_sa.source else {
            return false;
        };
        if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
            return false;
        }
    }
    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Card, AbilityKey.Cause);
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    // TODO: AbilityKey.Cause is a SpellAbility in Java, cannot be stored as String easily
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Discarded: " + Card + ", Cause: " + Cause
    format!(
        "Discarded: {}, Cause: {}",
        sa.get_triggering_object("Card").unwrap_or_default(),
        sa.get_triggering_object("Cause").unwrap_or_default()
    )
}
