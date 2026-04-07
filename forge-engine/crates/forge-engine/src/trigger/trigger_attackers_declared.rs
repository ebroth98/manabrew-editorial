use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_player_filter, matches_amount, matches_valid_card, TriggerMode};

fn attacked_target_matches(
    filter: &str,
    params: &RunParams,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) -> bool {
    params.attacked_card.is_some_and(|card_id| {
        super::trigger::matches_valid_card(filter, card_id, host_card, host_controller, game)
    }) || params.attacked_player.is_some_and(|player_id| {
        super::trigger::matches_valid_player(filter, player_id, host_controller)
    }) || params.defenders_card_ids.as_ref().is_some_and(|cards| {
        cards.iter().copied().any(|card_id| {
            super::trigger::matches_valid_card(filter, card_id, host_card, host_controller, game)
        })
    }) || params.defenders_player_ids.as_ref().is_some_and(|players| {
        players.iter().copied().any(|player_id| {
            super::trigger::matches_valid_player(filter, player_id, host_controller)
        })
    })
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::AttackersDeclared {
        valid_player,
        valid_attackers,
        valid_attackers_amount,
        attacked_target,
        ..
    } = mode
    {
        if !check_player_filter(
            valid_player,
            params.attacking_player.or(params.player),
            host_controller,
        ) {
            return false;
        }
        if let Some(filter) = attacked_target {
            if !attacked_target_matches(filter, params, host_card, host_controller, game) {
                return false;
            }
        }
        if valid_attackers.is_none()
            && valid_attackers_amount.is_none()
            && params.attacker_ids.is_none()
        {
            return true;
        }
        let Some(attacker_ids) = params.attacker_ids.as_ref() else {
            return false;
        };
        let matching_count = if let Some(filter) = valid_attackers {
            attacker_ids
                .iter()
                .filter(|&&attacker| {
                    matches_valid_card(filter, attacker, host_card, host_controller, game)
                })
                .count()
        } else {
            attacker_ids.len()
        };
        if let Some(amount_filter) = valid_attackers_amount {
            return matches_amount(amount_filter, matching_count);
        }
        return matching_count > 0;
    }
    panic!("Expected AttackersDeclared mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Attackers, attackers);
    if let Some(attacker_ids) = params.attacker_ids.as_ref() {
        let csv = attacker_ids
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Attackers", &csv);
    }
    // Java: sa.setTriggeringObject(AbilityKey.AttackedTarget, attackedTarget);
    // Combine defender players and defender cards into a single CSV
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
        if let Some(p) = params.attacked_player {
            if parts.is_empty() {
                parts.push(p.0.to_string());
            }
        } else if let Some(c) = params.attacked_card {
            if parts.is_empty() {
                parts.push(c.0.to_string());
            }
        }
        if !parts.is_empty() {
            sa.add_triggering_object("AttackedTarget", &parts.join(","));
        }
    }
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.AttackingPlayer);
    if let Some(p) = params.attacking_player {
        sa.add_triggering_object("AttackingPlayer", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Number Attackers: {}",
        sa.get_triggering_object("Attackers").unwrap_or("")
    )
}
