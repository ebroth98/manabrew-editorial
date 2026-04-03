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
    if let TriggerMode::DamageDone {
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
    panic!("Expected DamageDone mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Source, CardCopyService.getLKICopy(DamageSource))
    // TODO: Java uses CardCopyService.getLKICopy for the source. We just use the ID directly.
    if let Some(src) = params.damage_source {
        sa.add_triggering_object("Source", &src.0.to_string());
    }
    if let Some(card) = params.damage_target_card {
        sa.add_triggering_object("Target", &card.0.to_string());
    } else if let Some(player) = params.damage_target_player {
        sa.add_triggering_object("Target", &player.0.to_string());
    }
    // TODO: Java also sets Cause (SpellAbility) from runParams.
    // Skipping Cause for now since SpellAbility is complex and stored as object in Java.
    if let Some(amount) = params.damage_amount {
        sa.add_triggering_object("DamageAmount", &amount.to_string());
    }
    if let Some(p) = params.defending_player {
        sa.add_triggering_object("DefendingPlayer", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Damage Source: " + Source + ", Damaged: " + Target + ", Amount: " + DamageAmount
    format!(
        "Damage Source: {}, Damaged: {}, Amount: {}",
        sa.get_triggering_object("Source").unwrap_or(""),
        sa.get_triggering_object("Target").unwrap_or(""),
        sa.get_triggering_object("DamageAmount").unwrap_or("")
    )
}
