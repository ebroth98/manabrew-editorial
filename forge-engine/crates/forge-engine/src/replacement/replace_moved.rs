//! Replacement logic for `Event$ Moved`.
//!
//! Mirrors Java `ReplaceMoved.java` in `forge/game/replacement/`.

use forge_foundation::ZoneType;

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::{matches_valid_card, zone_matches, ReplacementEffect};
use super::replacement_handler::ReplacementEvent;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceMoved.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Moved {
        return false;
    }
    let (moving_id, origin, destination) = match event {
        ReplacementEvent::Moved {
            card,
            origin,
            destination,
        } => (*card, *origin, *destination),
        _ => return false,
    };
    if let Some(dest) = effect.params.get(keys::DESTINATION) {
        if !zone_matches(dest, destination) {
            return false;
        }
    }
    if let Some(orig) = effect.params.get(keys::ORIGIN) {
        if !zone_matches(orig, origin) {
            return false;
        }
    }
    let moving_card = &game.cards[moving_id.index()];
    if let Some(valid) = effect.params.get(keys::VALID_CARD) {
        if !matches_valid_card(valid, moving_card, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Moved.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let destination = match event {
        ReplacementEvent::Moved { destination, .. } => destination,
        _ => return ReplacementResult::NotReplaced,
    };
    if let Some(new_dest) = effect.params.get(keys::NEW_DESTINATION) {
        let new_zone = match new_dest.trim() {
            "Exile" => Some(ZoneType::Exile),
            "Graveyard" => Some(ZoneType::Graveyard),
            "Hand" => Some(ZoneType::Hand),
            "Library" => Some(ZoneType::Library),
            "Battlefield" => Some(ZoneType::Battlefield),
            _ => None,
        };
        if let Some(z) = new_zone {
            *destination = z;
            return ReplacementResult::Updated;
        }
    }
    ReplacementResult::Replaced
}
