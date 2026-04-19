//! Replacement logic for `Event$ RollDice`.
//!
//! Mirrors Java `ReplaceRollDice.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;

use super::replacement_effect::ReplacementEffect;
use crate::card_trait_base::CardTrait;
use super::replacement_handler::{
    execute_replace_effect_chain, execute_replace_with_numeric_update, resolve_replace_value,
    ReplacementEvent,
};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceRollDice.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::RollDice {
        return false;
    }
    let (player, sides) = match event {
        ReplacementEvent::RollDice { player, sides, .. } => (*player, *sides),
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !effect.matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    if let Some(valid_sides) = effect.params.get(keys::VALID_SIDES) {
        let rhs = resolve_replace_value(valid_sides, _game, source_card.id, event)
            .or_else(|| valid_sides.parse::<i32>().ok())
            .unwrap_or(0);
        if sides != rhs {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for RollDice.
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
    if let Some(result) =
        execute_replace_with_numeric_update(effect, _event, _game, _source_card_id, "Number")
    {
        return result;
    }
    if let Some(result) =
        execute_replace_with_numeric_update(effect, _event, _game, _source_card_id, "Ignore")
    {
        return result;
    }
    if let Some(replace_with) = effect.params.get(keys::REPLACE_WITH) {
        if let Some(result) = execute_replace_effect_chain(
            replace_with,
            _event,
            _game,
            _source_card_id,
            Some("IgnoreChosen"),
        ) {
            return result;
        }
        if let Some(result) = execute_replace_effect_chain(
            replace_with,
            _event,
            _game,
            _source_card_id,
            Some("DicePTExchanges"),
        ) {
            return result;
        }
    }
    ReplacementResult::Replaced
}
