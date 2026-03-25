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
    if params.attacked_card.is_none() {
        if let Some(players) = params.defenders_player_ids.as_deref() {
            let csv = players
                .iter()
                .map(|player_id| player_id.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            if !csv.is_empty() {
                sa.add_triggering_object("AttackedTarget", &csv);
            }
        }
    }
}
