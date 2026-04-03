use super::trigger::{check_card_filter, check_damage_target, TriggerMode};
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.damage_target_card {
        sa.add_triggering_object("Target", &card.0.to_string());
    } else if let Some(player) = params.damage_target_player {
        sa.add_triggering_object("Target", &player.0.to_string());
    }
    if let Some(src) = params.damage_source {
        sa.add_triggering_object("Source", &src.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: if Target != null { "Damaged: " + Target + ", " } + "Damage Source: " + Source
    let target = sa.get_triggering_object("Target").unwrap_or("");
    if target.is_empty() {
        format!(
            "Damage Source: {}",
            sa.get_triggering_object("Source").unwrap_or("")
        )
    } else {
        format!(
            "Damaged: {}, Damage Source: {}",
            target,
            sa.get_triggering_object("Source").unwrap_or("")
        )
    }
}
