use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let activator = params.get_cloned(keys::VALID_ACTIVATOR);
    let produced = params.get_cloned(keys::PRODUCED);
    TriggerMode::TapsForMana {
        valid_card,
        activator,
        produced,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::TapsForMana {
        valid_card,
        activator,
        produced,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = activator {
            let Some(player) = params.activator.or(params.player) else {
                return false;
            };
            if !super::trigger::matches_valid_player(filter, player, host_controller) {
                return false;
            }
        }
        if let Some(expected) = produced {
            let Some(actual) = params.produced.as_ref() else {
                return false;
            };
            if !actual.contains(expected) {
                return false;
            }
        }
        return true;
    }
    panic!("Expected TapsForMana mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(produced) = params.produced.as_ref() {
        sa.add_triggering_object("Produced", produced);
    }
    if let Some(p) = params.activator {
        sa.add_triggering_object("Activator", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "TappedForMana: {} Produced: {}",
        sa.trigger_objects
            .get("Card")
            .map(|s| s.as_str())
            .unwrap_or(""),
        sa.trigger_objects
            .get("Produced")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
