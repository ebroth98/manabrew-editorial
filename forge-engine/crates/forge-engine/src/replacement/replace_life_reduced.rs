//! Replacement logic for `Event$ LifeReduced`.
//!
//! Mirrors Java `ReplaceLifeReduced.java` in `forge/game/replacement/`.

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

/// Mirrors Java `ReplaceLifeReduced.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::LifeReduced {
        return false;
    }
    let (player, amount, is_damage) = match event {
        ReplacementEvent::LifeReduced {
            player,
            amount,
            is_damage,
        } => (*player, *amount, *is_damage),
        _ => return false,
    };
    if amount <= 0 {
        return false;
    }
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !effect.matches_valid_player(valid, player, source_card) {
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
    if let Some(result_cmp) = effect.params.get(keys::RESULT) {
        let final_life = _game.player(player).life - amount;
        let rhs = result_cmp
            .get(2..)
            .and_then(|n| n.parse::<i32>().ok())
            .unwrap_or(0);
        let cmp = format!("{}{}", result_cmp.get(..2).unwrap_or("GE"), rhs);
        if !compare_expr(final_life, &cmp) {
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
    match event {
        ReplacementEvent::LifeReduced { .. } => {}
        _ => return ReplacementResult::NotReplaced,
    }
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
    {
        if let ReplacementEvent::LifeReduced { amount, .. } = event {
            *amount = 0;
        }
        return ReplacementResult::Prevented;
    }
    if let Some(result) =
        execute_replace_with_numeric_update(effect, event, _game, _source_card_id, "Amount")
    {
        return result;
    }
    ReplacementResult::Replaced
}
