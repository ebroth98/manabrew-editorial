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
    if let TriggerMode::AttackerBlockedByCreature {
        valid_card,
        valid_blocked,
    } = mode
    {
        return check_card_filter(valid_card, params.blocker, host_card, host_controller, game)
            && check_card_filter(
                valid_blocked,
                params.blocked_attacker,
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected AttackerBlockedByCreature mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(attacker) = params.attacker {
        sa.add_triggering_object("Attacker", &attacker.0.to_string());
    }
    if let Some(blocker) = params.blocker {
        sa.add_triggering_object("Blocker", &blocker.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Attacker: {}, Blocker: {}",
        sa.get_triggering_object("Attacker").unwrap_or(""),
        sa.get_triggering_object("Blocker").unwrap_or("")
    )
}
