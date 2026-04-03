use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_damage_target, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::DamageAll {
        valid_source,
        valid_target,
    } = mode
    {
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
            false,
        );
    }
    panic!("Expected DamageAll mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sets DamageAmount (total), Sources (set of source cards), Targets (set of target entities)
    // from filtered CardDamageMap. We approximate with single source/target from params.
    // TODO: Java filters the damage map by ValidSource/ValidTarget and computes totals.
    if let Some(amount) = params.damage_amount {
        sa.add_triggering_object("DamageAmount", &amount.to_string());
    }
    if let Some(src) = params.damage_source {
        sa.add_triggering_object("Sources", &src.0.to_string());
    }
    if let Some(card) = params.damage_target_card {
        sa.add_triggering_object("Targets", &card.0.to_string());
    } else if let Some(player) = params.damage_target_player {
        sa.add_triggering_object("Targets", &player.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Damage Source: " + Sources + ", Damaged: " + Targets + ", Amount: " + DamageAmount
    format!(
        "Damage Source: {}, Damaged: {}, Amount: {}",
        sa.get_triggering_object("Sources").unwrap_or(""),
        sa.get_triggering_object("Targets").unwrap_or(""),
        sa.get_triggering_object("DamageAmount").unwrap_or("")
    )
}
