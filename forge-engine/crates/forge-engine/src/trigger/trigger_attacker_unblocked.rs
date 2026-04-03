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
    if let TriggerMode::AttackerUnblocked { valid_card } = mode {
        return check_card_filter(
            valid_card,
            params.attacker,
            host_card,
            host_controller,
            game,
        );
    }
    panic!("Expected AttackerUnblocked mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(attacker) = params.attacker {
        sa.add_triggering_object("Attacker", &attacker.0.to_string());
    }
    if let Some(c) = params.attacked_card {
        sa.add_triggering_object("Defender", &c.0.to_string());
    } else if let Some(p) = params.attacked_player {
        sa.add_triggering_object("Defender", &p.0.to_string());
    }
    if let Some(p) = params.defending_player {
        sa.add_triggering_object("DefendingPlayer", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Attacker: {}",
        sa.get_triggering_object("Attacker").unwrap_or("")
    )
}
