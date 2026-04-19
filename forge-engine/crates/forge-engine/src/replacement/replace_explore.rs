//! Replacement logic for `Event$ Explore`.
//!
//! Mirrors Java `ReplaceExplore.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::{matches_valid_card, ReplacementEffect};
use super::replacement_handler::ReplacementEvent;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceExplore.canReplace()`.
/// Java checks `ValidExplorer` using the same pattern as `ValidCard`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Explore {
        return false;
    }
    let card = match event {
        ReplacementEvent::Explore { card } => *card,
        _ => return false,
    };
    let target_card = &game.cards[card.index()];
    // Java uses ValidExplorer with the same semantics as ValidCard.
    if let Some(valid) = effect
        .params
        .get(keys::VALID_EXPLORER)
        .or(effect.params.get(keys::VALID_CARD))
    {
        if !matches_valid_card(effect, valid, target_card, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Explore.
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
    ReplacementResult::Replaced
}
