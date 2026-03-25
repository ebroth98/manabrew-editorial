use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_damage_target, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_source = params.get_cloned(keys::VALID_SOURCE);
    let valid_target = params.get_cloned(keys::VALID_TARGET);
    let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
    TriggerMode::DamageDealtOnce {
        valid_source,
        valid_target,
        combat_damage_only,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::DamageDealtOnce {
        valid_source,
        valid_target,
        combat_damage_only,
    } = mode
    {
        if *combat_damage_only && params.is_combat_damage != Some(true) {
            return false;
        }
        return check_card_filter(
            valid_source,
            params.damage_source,
            host_card,
            host_controller,
            game,
        ) && check_damage_target(
            valid_target,
            params,
            host_card,
            host_controller,
            game,
            true,
        );
    }
    panic!("Expected DamageDealtOnce mode");
}
