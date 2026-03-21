//! Replacement logic for `Event$ DrawCards`.
//!
//! Mirrors Java `ReplaceDrawCards.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceDrawCards.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::DrawCards {
        return false;
    }
    let (player, count) = match event {
        ReplacementEvent::DrawCards { player, count } => (*player, *count),
        _ => return false,
    };
    if count <= 0 {
        return false;
    }
    if let Some(valid) = effect.params.get("ValidPlayer") {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for DrawCards.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let count = match event {
        ReplacementEvent::DrawCards { count, .. } => count,
        _ => return ReplacementResult::NotReplaced,
    };
    if effect
        .params
        .get("Prevent")
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        *count = 0;
        return ReplacementResult::Prevented;
    }
    ReplacementResult::Replaced
}
