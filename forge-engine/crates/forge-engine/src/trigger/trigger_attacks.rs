use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let alone = params.is_true(keys::ALONE);
    TriggerMode::Attacks { valid_card, alone }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Attacks { valid_card, alone } = mode {
        if *alone && params.num_attackers.unwrap_or(0) != 1 {
            return false;
        }
        return check_card_filter(
            valid_card,
            params.attacker,
            host_card,
            host_controller,
            game,
        );
    }
    panic!("Expected Attacks mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Defender, runParams.get(AbilityKey.Attacked));
    if let Some(p) = params.attacked_player {
        sa.add_triggering_object("Defender", &p.0.to_string());
    } else if let Some(c) = params.attacked_card {
        sa.add_triggering_object("Defender", &c.0.to_string());
    }
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.Attacker, AbilityKey.Defenders, AbilityKey.DefendingPlayer);
    if let Some(attacker) = params.attacker {
        sa.add_triggering_object("Attacker", &attacker.0.to_string());
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
