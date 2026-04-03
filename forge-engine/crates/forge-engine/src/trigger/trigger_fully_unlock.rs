use super::trigger::{check_card_filter, check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::FullyUnlock {
        valid_card,
        valid_player,
    } = mode
    else {
        panic!("Expected FullyUnlock mode");
    };
    check_card_filter(valid_card, params.card, host_card, host_controller, game)
        && check_player_filter(valid_player, params.player, host_controller)
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    TriggerMode::FullyUnlock {
        valid_card,
        valid_player,
    }
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}, Card: {}",
        sa.get_triggering_object("Player").unwrap_or_default(),
        sa.get_triggering_object("Card").unwrap_or_default()
    )
}
