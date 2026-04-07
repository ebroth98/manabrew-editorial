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
    if let TriggerMode::AttackerBlocked { valid_card } = mode {
        return check_card_filter(
            valid_card,
            params.attacker,
            host_card,
            host_controller,
            game,
        );
    }
    panic!("Expected AttackerBlocked mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(attacker) = params.attacker {
        sa.add_triggering_object("Attacker", &attacker.0.to_string());
    }
    if let Some(blockers) = params.blocker_ids.as_ref() {
        let csv = blockers
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Blockers", &csv);
    }
    if let Some(p) = params.defending_player {
        sa.add_triggering_object("DefendingPlayer", &p.0.to_string());
    }
    if let Some(c) = params.attacked_card {
        sa.add_triggering_object("Defender", &c.0.to_string());
    } else if let Some(p) = params.attacked_player {
        sa.add_triggering_object("Defender", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    let attacker = sa.get_triggering_object("Attacker").unwrap_or("");
    let num_blockers = sa
        .get_triggering_object("Blockers")
        .map(|s| {
            if s.is_empty() {
                0
            } else {
                s.split(',').count()
            }
        })
        .unwrap_or(0);
    format!("Attacker: {}, Number Blockers: {}", attacker, num_blockers)
}
