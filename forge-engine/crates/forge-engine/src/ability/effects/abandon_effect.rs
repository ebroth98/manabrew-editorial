//! Abandon — leave a game in a multiplayer match.
//! Ported from Java's AbandonEffect: sets player as lost.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    ctx.game.player_mut(controller).has_lost = true;
}
