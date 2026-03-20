//! Replacement logic for `Event$ Cascade`.
//!
//! Mirrors Java `ReplaceCascade.java` in `forge/game/replacement/`.
//!
//! TODO: implement — currently returns `false` from `can_replace`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::ReplacementEffect;
use super::replacement_result::ReplacementResult;

/// Stub — always returns `false`. TODO: implement.
pub fn can_replace(
    _effect: &ReplacementEffect,
    _event: &ReplacementEvent,
    _game: &GameState,
    _source_card: &CardInstance,
) -> bool {
    false
}

/// Stub — returns `NotReplaced`. TODO: implement.
pub fn execute(
    _effect: &ReplacementEffect,
    _event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    ReplacementResult::NotReplaced
}
