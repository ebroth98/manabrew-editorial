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

    ctx.game.player_exchange_life_totals(controller, target);
}
