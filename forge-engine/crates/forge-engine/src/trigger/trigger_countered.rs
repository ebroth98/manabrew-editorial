use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, matches_valid_card, matches_valid_sa, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Countered {
        valid_card,
        valid_cause,
        valid_sa,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = valid_cause {
            if let Some(cause) = params.cause.as_ref() {
                let Some(cause_card) = cause.source else {
                    return false;
                };
                if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(filter) = valid_sa {
            if let Some(countered_sa) = params.spell_ability.as_ref() {
                if !matches_valid_sa(filter, countered_sa) {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }
    panic!("Expected Countered mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Card, AbilityKey.Cause, AbilityKey.CounteredSA)
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    // TODO: Java also sets Cause (SpellAbility) and CounteredSA (SpellAbility) from runParams.
    // Skipping Cause and CounteredSA for now since SpellAbility is complex and stored as object in Java.
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Countered: " + Card + ", Cause: " + Cause
    format!(
        "Countered: {}, Cause: {}",
        sa.get_triggering_object("Card").unwrap_or(""),
        sa.get_triggering_object("Cause").unwrap_or("")
    )
}
