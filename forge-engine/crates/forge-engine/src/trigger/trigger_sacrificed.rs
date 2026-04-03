use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_player_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    TriggerMode::Sacrificed {
        valid_card,
        valid_player,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Sacrificed {
        valid_card,
        valid_player,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_player_filter(valid_player, params.player, host_controller);
    }
    panic!("Expected Sacrificed mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Sacrificed: {}",
        sa.trigger_objects
            .get("Card")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
