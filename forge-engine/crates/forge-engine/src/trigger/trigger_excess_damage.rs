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
    if let TriggerMode::ExcessDamage {
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
    panic!("Expected ExcessDamage mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.damage_target_card {
        sa.add_triggering_object("Target", &card.0.to_string());
    } else if let Some(player) = params.damage_target_player {
        sa.add_triggering_object("Target", &player.0.to_string());
    }
    if let Some(amount) = params.damage_amount {
        sa.add_triggering_object("DamageAmount", &amount.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Damaged: " + Target + ", Amount: " + DamageAmount
    format!(
        "Damaged: {}, Amount: {}",
        sa.get_triggering_object("Target").unwrap_or_default(),
        sa.get_triggering_object("DamageAmount").unwrap_or_default()
    )
}
