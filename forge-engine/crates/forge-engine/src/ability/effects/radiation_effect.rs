//! Radiation — give radiation counters (Fallout).

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);
    let target = sa
        .target_chosen
        .target_player
        .unwrap_or(sa.activating_player);
    ctx.game.player_mut(target).radiation_counters += amount;
}
