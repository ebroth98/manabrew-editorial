//! Replacement logic for `Event$ Mill`.
//!
//! Mirrors Java `ReplaceMill.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::ReplacementEffect;
use crate::card_trait_base::CardTrait;
use super::replacement_handler::{execute_replace_with_numeric_update, ReplacementEvent};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceMill.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Mill {
        return false;
    }
    let player = match event {
        ReplacementEvent::Mill { player, .. } => *player,
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !effect.matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Mill.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    match event {
        ReplacementEvent::Mill { .. } => {}
        _ => return ReplacementResult::NotReplaced,
    }
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        if let ReplacementEvent::Mill { count, .. } = event {
            *count = 0;
        }
        return ReplacementResult::Prevented;
    }
    if let Some(result) =
        execute_replace_with_numeric_update(effect, event, _game, _source_card_id, "Number")
    {
        return result;
    }
    ReplacementResult::Replaced
}
