use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::DamagePreventedOnce { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected DamagePreventedOnce mode");
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
    // Java: "Damage Target: " + Target + ", Amount: " + DamageAmount
    format!(
        "Damage Target: {}, Amount: {}",
        sa.get_triggering_object("Target").unwrap_or(""),
        sa.get_triggering_object("DamageAmount").unwrap_or("")
    )
}
