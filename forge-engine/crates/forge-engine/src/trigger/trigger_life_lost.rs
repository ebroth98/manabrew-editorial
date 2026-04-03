use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    let first_time_only = params.has("FirstTime");
    TriggerMode::LifeLost {
        valid_player,
        first_time_only,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::LifeLost {
        valid_player,
        first_time_only,
    } = mode
    {
        if !check_player_filter(valid_player, params.player, host_controller) {
            return false;
        }
        if *first_time_only && params.first_time != Some(true) {
            return false;
        }
        return true;
    }
    panic!("Expected LifeLost mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(amount) = params.life_amount {
        sa.add_triggering_object("LifeAmount", &amount.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Player: " + Player + ", LostAmount: " + LifeAmount
    format!(
        "Player: {}, LostAmount: {}",
        sa.get_triggering_object("Player").unwrap_or_default(),
        sa.get_triggering_object("LifeAmount").unwrap_or_default()
    )
}
