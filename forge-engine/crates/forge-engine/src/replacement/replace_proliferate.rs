//! Replacement logic for `Event$ Proliferate`.
//!
//! Mirrors Java `ReplaceProliferate.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::parsing::keys;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceProliferate.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Proliferate {
        return false;
    }
    let (player, count) = match event {
        ReplacementEvent::Proliferate { player, count } => (*player, *count),
        _ => return false,
    };
    if count <= 0 {
        return false;
    }
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Proliferate.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let count = match event {
        ReplacementEvent::Proliferate { count, .. } => count,
        _ => return ReplacementResult::NotReplaced,
    };
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        *count = 0;
        return ReplacementResult::Prevented;
    }
    ReplacementResult::Replaced
}
