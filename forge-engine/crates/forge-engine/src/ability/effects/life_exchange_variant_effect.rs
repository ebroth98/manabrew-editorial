//! LifeExchangeVariant effect — variant life total exchange.
//!
//! Ported from Java's `LifeExchangeVariantEffect.java`.
//! Exchange life totals with specific modifications.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let target = sa
        .target_chosen
        .target_player
        .unwrap_or_else(|| ctx.game.opponent_of(controller));

    let life1 = ctx.game.player(controller).life;
    let life2 = ctx.game.player(target).life;

    ctx.game.player_mut(controller).life = life2;
    ctx.game.player_mut(target).life = life1;

    // Track life gained/lost
    if life2 > life1 {
        ctx.game.player_mut(controller).life_gained_this_turn += life2 - life1;
    } else {
        ctx.game.player_mut(controller).life_lost_this_turn += life1 - life2;
    }
    if life1 > life2 {
        ctx.game.player_mut(target).life_gained_this_turn += life1 - life2;
    } else {
        ctx.game.player_mut(target).life_lost_this_turn += life2 - life1;
    }
}
