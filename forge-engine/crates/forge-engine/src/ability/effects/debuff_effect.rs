//! Debuff — reduce stats permanently (digital-only).

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);
    if let Some(target) = sa.target_chosen.target_card {
        ctx.game.card_mut(target).power_modifier -= amount;
        ctx.game.card_mut(target).toughness_modifier -= amount;
    }
}
