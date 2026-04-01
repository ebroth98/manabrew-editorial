//! Replacement logic for `Event$ Draw`.
//!
//! Mirrors Java `ReplaceDraw.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_handler::ReplacementEvent;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceDraw.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Draw {
        return false;
    }
    let player = match event {
        ReplacementEvent::Draw { player, .. } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    // NotFirstCardInDrawStep$ True: only replace draws that are NOT the first in the draw step.
    // Used by Alhammarret's Archive to skip its first draw in the draw step.
    if effect.params.get("NotFirstCardInDrawStep").map(|v| v == "True").unwrap_or(false) {
        if let ReplacementEvent::Draw { is_first_in_draw_step, .. } = event {
            if *is_first_in_draw_step {
                return false;
            }
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Draw.
pub fn execute(
    effect: &ReplacementEffect,
    _event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
        || effect.params.has(keys::SKIP)
    {
        return ReplacementResult::Skipped;
    }
    // ReplaceWith$ DrawTwo — draw an extra card (Alhammarret's Archive).
    if let Some(replace) = effect.params.get(keys::REPLACE_WITH) {
        if replace == "DrawTwo" || replace == "DrawExtra" {
            if let ReplacementEvent::Draw { extra_draws, .. } = _event {
                *extra_draws += 1;
                return ReplacementResult::Updated;
            }
        }
    }
    ReplacementResult::Replaced
}
