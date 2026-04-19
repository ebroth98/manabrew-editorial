//! Replacement logic for `Event$ DrawCards`.
//!
//! Mirrors Java `ReplaceDrawCards.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;

use super::replacement_effect::ReplacementEffect;
use crate::card_trait_base::CardTrait;
use super::replacement_handler::{execute_replace_with_numeric_update, ReplacementEvent};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceDrawCards.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::DrawCards {
        return false;
    }
    let (player, count) = match event {
        ReplacementEvent::DrawCards { player, count } => (*player, *count),
        _ => return false,
    };
    if count <= 0 {
        return false;
    }
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !effect.matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    if let Some(number_cmp) = effect.params.get(keys::NUMBER) {
        let rhs = number_cmp
            .get(2..)
            .and_then(|n| n.parse::<i32>().ok())
            .unwrap_or(0);
        let cmp = format!("{}{}", number_cmp.get(..2).unwrap_or("GE"), rhs);
        if !compare_expr(count, &cmp) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for DrawCards.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    match event {
        ReplacementEvent::DrawCards { .. } => {}
        _ => return ReplacementResult::NotReplaced,
    }
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        if let ReplacementEvent::DrawCards { count, .. } = event {
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
