use super::trigger::{check_card_filter, check_damage_target, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::DamageDoneOnceByController {
        valid_source,
        valid_target,
        combat_damage_only,
    } = mode
    else {
        panic!("Expected DamageDoneOnceByController mode");
    };

    if *combat_damage_only && params.is_combat_damage != Some(true) {
        return false;
    }
    if !check_damage_target(valid_target, params, host_card, host_controller, game, true) {
        return false;
    }
    check_card_filter(
        valid_source,
        params.damage_source,
        host_card,
        host_controller,
        game,
    )
}
