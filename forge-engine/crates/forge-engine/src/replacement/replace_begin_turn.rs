//! Replacement logic for `Event$ BeginTurn`.
//!
//! Mirrors Java `ReplaceBeginTurn.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceBeginTurn.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::BeginTurn {
        return false;
    }
    let player = match event {
        ReplacementEvent::BeginTurn { player } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get("ValidPlayer") {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for BeginTurn.
pub fn execute(
    effect: &ReplacementEffect,
    _event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    if effect
        .params
        .get("Prevent")
        .map(|s| s == "True")
        .unwrap_or(false)
        || effect.params.contains_key("Skip")
    {
        return ReplacementResult::Skipped;
    }
    ReplacementResult::Replaced
}
