//! Replacement logic for `Event$ DamageDone`.
//!
//! Mirrors Java `ReplaceDamage.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::parsing::keys;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::ReplacementEffect;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceDamage.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    _source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::DamageDone {
        return false;
    }
    let target_is_player = matches!(event, ReplacementEvent::DamageToPlayer { .. });
    let amount = match event {
        ReplacementEvent::DamageToCard { amount, .. } => *amount,
        ReplacementEvent::DamageToPlayer { amount, .. } => *amount,
        _ => return false,
    };
    if amount <= 0 {
        return false;
    }
    if let Some(valid_target) = effect.params.get(keys::VALID_TARGET) {
        let target_matches = match valid_target.trim() {
            "Player" => target_is_player,
            "Card" | "Creature" | "Permanent" => !target_is_player,
            "Any" | "CardOrPlayer" => true,
            _ => false,
        };
        if !target_matches {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for DamageDone.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let amount = match event {
        ReplacementEvent::DamageToCard { amount, .. } => amount,
        ReplacementEvent::DamageToPlayer { amount, .. } => amount,
        _ => return ReplacementResult::NotReplaced,
    };
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        *amount = 0;
        return ReplacementResult::Prevented;
    }
    ReplacementResult::Replaced
}
