//! Replacement logic for `Event$ DamageDone`.
//!
//! Mirrors Java `ReplaceDamage.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;

use super::replacement_effect::ReplacementEffect;
use super::replacement_handler::ReplacementEvent;
use super::replacement_handler::{execute_replace_with_numeric_update, resolve_replace_value};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

fn matches_valid_card_list(
    effect: &ReplacementEffect,
    expr: &str,
    card: &Card,
    source_card: &Card,
) -> bool {
    super::replacement_effect::matches_valid_card(effect, expr, card, source_card)
}

fn matches_valid_player_list(
    effect: &ReplacementEffect,
    expr: &str,
    player: crate::ids::PlayerId,
    source_card: &Card,
) -> bool {
    super::replacement_effect::matches_valid_player(effect, expr, player, source_card)
}

/// Mirrors Java `ReplaceDamage.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::DamageDone {
        return false;
    }
    let (damage_source, target_player, target_card, amount, is_combat) = match event {
        ReplacementEvent::DamageToCard {
            target,
            amount,
            source,
            is_combat,
        } => (*source, None, Some(*target), *amount, *is_combat),
        ReplacementEvent::DamageToPlayer {
            target,
            amount,
            source,
            is_combat,
        } => (*source, Some(*target), None, *amount, *is_combat),
        _ => return false,
    };
    if amount <= 0 {
        return false;
    }
    if let Some(valid_source) = effect.params.get(keys::VALID_SOURCE) {
        let Some(source_id) = damage_source else {
            return false;
        };
        if !matches_valid_card_list(effect, valid_source, game.card(source_id), source_card) {
            return false;
        }
    }
    if let Some(valid_target) = effect.params.get(keys::VALID_TARGET) {
        let target_matches = if let Some(target) = target_player {
            matches_valid_player_list(effect, valid_target, target, source_card)
        } else if let Some(target) = target_card {
            matches_valid_card_list(effect, valid_target, game.card(target), source_card)
        } else {
            false
        };
        if !target_matches {
            return false;
        }
    }
    if let Some(max_speed) = effect.params.get("MaxSpeed") {
        let wants_max_speed = max_speed.eq_ignore_ascii_case("true");
        if wants_max_speed != (game.player(source_card.controller).speed == 4) {
            return false;
        }
    }
    if let Some(is_combat_param) = effect.params.get(keys::IS_COMBAT) {
        let wants_combat = is_combat_param.eq_ignore_ascii_case("true");
        if wants_combat != is_combat {
            return false;
        }
    }
    if let Some(damage_amount) = effect.params.get(keys::DAMAGE_AMOUNT) {
        let threshold = damage_amount.get(2..).unwrap_or("");
        let rhs = resolve_replace_value(threshold, game, source_card.id, event)
            .or_else(|| threshold.parse::<i32>().ok())
            .unwrap_or(0);
        let cmp = format!("{}{}", damage_amount.get(..2).unwrap_or("GE"), rhs);
        if !compare_expr(amount, &cmp) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for DamageDone.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    game: &GameState,
    source_card_id: CardId,
) -> ReplacementResult {
    match event {
        ReplacementEvent::DamageToCard { .. } | ReplacementEvent::DamageToPlayer { .. } => {}
        _ => return ReplacementResult::NotReplaced,
    }
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        match event {
            ReplacementEvent::DamageToCard { amount, .. } => *amount = 0,
            ReplacementEvent::DamageToPlayer { amount, .. } => *amount = 0,
            _ => {}
        }
        return ReplacementResult::Prevented;
    }
    // Handle built-in replacement modes before SVar chain.
    if let Some(replace) = effect.params.get(keys::REPLACE_WITH) {
        match replace {
            "DmgTwice" | "DoubleDamage" => {
                match event {
                    ReplacementEvent::DamageToCard { amount, .. } => *amount *= 2,
                    ReplacementEvent::DamageToPlayer { amount, .. } => *amount *= 2,
                    _ => {}
                }
                return ReplacementResult::Updated;
            }
            "DmgHalf" | "HalfDamage" => {
                match event {
                    ReplacementEvent::DamageToCard { amount, .. } => *amount = (*amount + 1) / 2,
                    ReplacementEvent::DamageToPlayer { amount, .. } => *amount = (*amount + 1) / 2,
                    _ => {}
                }
                return ReplacementResult::Updated;
            }
            "DmgPlus1" => {
                match event {
                    ReplacementEvent::DamageToCard { amount, .. } => *amount += 1,
                    ReplacementEvent::DamageToPlayer { amount, .. } => *amount += 1,
                    _ => {}
                }
                return ReplacementResult::Updated;
            }
            "DmgPlus2" => {
                match event {
                    ReplacementEvent::DamageToCard { amount, .. } => *amount += 2,
                    ReplacementEvent::DamageToPlayer { amount, .. } => *amount += 2,
                    _ => {}
                }
                return ReplacementResult::Updated;
            }
            _ => {}
        }
    }
    if let Some(result) =
        execute_replace_with_numeric_update(effect, event, game, source_card_id, "DamageAmount")
    {
        return result;
    }
    ReplacementResult::Replaced
}
