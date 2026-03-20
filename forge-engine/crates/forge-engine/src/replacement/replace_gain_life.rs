//! Replacement logic for `Event$ GainLife`.
//!
//! Mirrors Java `ReplaceGainLife.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceGainLife.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::GainLife {
        return false;
    }
    let player = match event {
        ReplacementEvent::GainLife { player, .. } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get("ValidPlayer") {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for GainLife.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let amount = match event {
        ReplacementEvent::GainLife { amount, .. } => amount,
        _ => return ReplacementResult::NotReplaced,
    };
    if effect
        .params
        .get("Prevent")
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        *amount = 0;
        return ReplacementResult::Skipped;
    }
    if let Some(replace) = effect.params.get("ReplaceWith") {
        if replace == "GainDouble" {
            *amount *= 2;
            return ReplacementResult::Updated;
        }
    }
    ReplacementResult::Replaced
}
