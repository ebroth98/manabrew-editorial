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
    if let TriggerMode::AttackerUnblockedOnce { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected AttackerUnblockedOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.AttackingPlayer, AbilityKey.Defenders);
    if let Some(p) = params.attacking_player {
        sa.add_triggering_object("AttackingPlayer", &p.0.to_string());
    }
    // Defenders combines both player and card defender IDs
    {
        let mut parts = Vec::new();
        if let Some(players) = params.defenders_player_ids.as_ref() {
            for p in players {
                parts.push(p.0.to_string());
            }
        }
        if let Some(cards) = params.defenders_card_ids.as_ref() {
            for c in cards {
                parts.push(c.0.to_string());
            }
        }
        if !parts.is_empty() {
            sa.add_triggering_object("Defenders", &parts.join(","));
        }
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "AttackingPlayer: {}, Defenders: {}",
        sa.get_triggering_object("AttackingPlayer").unwrap_or(""),
        sa.get_triggering_object("Defenders").unwrap_or("")
    )
}
