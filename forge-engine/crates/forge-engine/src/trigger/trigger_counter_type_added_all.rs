use super::trigger::{matches_valid_card, matches_valid_player, TriggerMode};
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
    let TriggerMode::CounterTypeAddedAll {
        valid_object,
        first_time_only,
    } = mode
    else {
        panic!("Expected CounterTypeAddedAll mode");
    };

    if let Some(filter) = valid_object {
        let object_ok = if let Some(cid) = params.object_card.or(params.card) {
            matches_valid_card(filter, cid, host_card, host_controller, game)
        } else if let Some(pid) = params.object_player.or(params.player) {
            matches_valid_player(filter, pid, host_controller)
        } else {
            false
        };
        if !object_ok {
            return false;
        }
    }

    if *first_time_only && params.first_time != Some(true) {
        return false;
    }

    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(obj) = params.object_card {
        sa.add_triggering_object("Object", &obj.0.to_string());
    } else if let Some(p) = params.object_player {
        sa.add_triggering_object("Object", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "AddedOnce: {}",
        sa.trigger_objects
            .get("Object")
            .cloned()
            .unwrap_or_default()
    )
}
