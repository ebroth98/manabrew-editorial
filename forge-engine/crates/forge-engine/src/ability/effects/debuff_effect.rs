//! Debuff — reduce stats permanently (digital-only).

use super::EffectContext;
use crate::spellability::SpellAbility;

/// End-of-turn revert for debuff. Mirrors the `GameCommand.run()` in Java
/// `DebuffEffect` that restores the original P/T when the effect expires.
///
/// Reverses the debuff by adding back the debuff amount to power/toughness modifiers.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId, amount: i32) {
    if game.card(card_id).zone == forge_foundation::ZoneType::Battlefield {
        game.card_mut(card_id).power_modifier += amount;
        game.card_mut(card_id).toughness_modifier += amount;
    }
}

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);
    if let Some(target) = sa.target_chosen.target_card {
        ctx.game.card_mut(target).power_modifier -= amount;
        ctx.game.card_mut(target).toughness_modifier -= amount;
    }
}
