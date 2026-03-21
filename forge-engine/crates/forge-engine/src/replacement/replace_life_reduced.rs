//! Replacement logic for `Event$ LifeReduced`.
//!
//! Mirrors Java `ReplaceLifeReduced.java` in `forge/game/replacement/`.

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::game::GameState;
use crate::ids::CardId;

use super::replacement_handler::ReplacementEvent;
use super::replacement_effect::{matches_valid_player, ReplacementEffect};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceLifeReduced.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &CardInstance,
) -> bool {
    if effect.event != ReplacementType::LifeReduced {
        return false;
    }
    let (player, _amount, is_damage) = match event {
        ReplacementEvent::LifeReduced {
            player,
            amount,
            is_damage,
        } => (*player, *amount, *is_damage),
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    // Check IsDamage filter if present.
    if let Some(is_dmg) = effect.params.get(keys::IS_DAMAGE) {
        let expected = is_dmg.eq_ignore_ascii_case("True");
        if is_damage != expected {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for LifeReduced.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    let amount = match event {
        ReplacementEvent::LifeReduced { amount, .. } => amount,
        _ => return ReplacementResult::NotReplaced,
    };
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        *amount = 0;
        return ReplacementResult::Prevented;
    }
    ReplacementResult::Replaced
}
