//! Replacement logic for `Event$ AssignDealDamage`.
//!
//! Mirrors Java `ReplaceAssignDealDamage.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_card, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceAssignDealDamage.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::AssignDealDamage {
        return false;
    }
    let card = match event {
        ReplacementEvent::AssignDealDamage { card } => *card,
        _ => return false,
    };
    let target_card = &game.cards[card.index()];
    if let Some(valid) = effect.params.get("ValidCard") {
        if !matches_valid_card(valid, target_card, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for AssignDealDamage.
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
