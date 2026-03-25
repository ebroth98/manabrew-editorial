//! Replacement logic for `Event$ AddCounter`.
//!
//! Mirrors Java `ReplaceAddCounter.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::{matches_valid_card, ReplacementEffect};
use super::replacement_handler::ReplacementEvent;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceAddCounter.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::AddCounter {
        return false;
    }
    let target = match event {
        ReplacementEvent::AddCounter { target, .. } => *target,
        _ => return false,
    };
    let target_card = &game.cards[target.index()];
    if let Some(valid) = effect.params.get(keys::VALID_CARD) {
        if !matches_valid_card(valid, target_card, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for AddCounter.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let count = match event {
        ReplacementEvent::AddCounter { count, .. } => count,
        _ => return ReplacementResult::NotReplaced,
    };
    if let Some(replace) = effect.params.get(keys::REPLACE_WITH) {
        match replace {
            "AddOneMoreCounter" | "AddOneMoreCounters" => {
                *count += 1;
                return ReplacementResult::Updated;
            }
            "AddTwiceCounters" | "DoubleCounters" => {
                *count *= 2;
                return ReplacementResult::Updated;
            }
            _ => {
                eprintln!(
                    "[WARN] Unknown replacement mode in AddCounter event: {:?}",
                    replace
                );
            }
        }
    }
    ReplacementResult::Replaced
}
