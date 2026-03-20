//! Replacement logic for `Event$ Counter` (countering a spell).
//!
//! Mirrors Java `ReplaceCounter.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_card, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceCounter.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::Counter {
        return false;
    }
    let target_id = match event {
        ReplacementEvent::Counter { card } => *card,
        _ => return false,
    };
    let target_card = &game.cards[target_id.index()];
    if let Some(valid) = effect.params.get("ValidCard") {
        if !matches_valid_card(valid, target_card, source_card) {
            return false;
        }
    }
    true
}

/// CantHappen layer prevents countering (e.g. "can't be countered").
pub fn execute(
    _effect: &ReplacementEffect,
    _event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    ReplacementResult::Replaced
}
