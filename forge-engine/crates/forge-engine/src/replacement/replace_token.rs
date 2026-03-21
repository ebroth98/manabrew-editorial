//! Replacement logic for `Event$ CreateToken`.
//!
//! Mirrors Java `ReplaceToken.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceToken.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::CreateToken {
        return false;
    }
    let player = match event {
        ReplacementEvent::CreateToken { player, .. } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for CreateToken.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let count = match event {
        ReplacementEvent::CreateToken { count, .. } => count,
        _ => return ReplacementResult::NotReplaced,
    };
    if let Some(replace) = effect.params.get(keys::REPLACE_WITH) {
        if replace == "DoubleToken" {
            *count *= 2;
            return ReplacementResult::Updated;
        }
    }
    ReplacementResult::Replaced
}
