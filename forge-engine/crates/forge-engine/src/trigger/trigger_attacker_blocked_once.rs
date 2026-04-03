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
    if let TriggerMode::AttackerBlockedOnce { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected AttackerBlockedOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(attackers) = params.attacker_ids.as_ref() {
        let csv = attackers.iter().map(|c| c.0.to_string()).collect::<Vec<_>>().join(",");
        sa.add_triggering_object("Attackers", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Attackers: {}",
        sa.get_triggering_object("Attackers").unwrap_or("")
    )
}
