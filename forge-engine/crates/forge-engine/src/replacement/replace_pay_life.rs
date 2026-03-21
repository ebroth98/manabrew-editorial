//! Replacement logic for `Event$ PayLife`.
//!
//! Mirrors Java `ReplacePayLife.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplacePayLife.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::PayLife {
        return false;
    }
    let player = match event {
        ReplacementEvent::PayLife { player, .. } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for PayLife.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let amount = match event {
        ReplacementEvent::PayLife { amount, .. } => amount,
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
