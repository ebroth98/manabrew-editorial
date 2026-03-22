//! LosePerpetual — remove perpetual effects (digital-only, Alchemy).
//! Ported from Java's LosePerpetualEffect: removes a perpetual trait change
//! identified by the triggering trigger's timestamp.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Digital-only: remove a perpetual effect from the host card.
    // In our engine, perpetual effects are tracked as svars on the card.
    if let Some(source) = sa.source {
        // Remove perpetual markers — clear any perpetual-prefixed svars
        let perpetual_keys: Vec<String> = ctx
            .game
            .card(source)
            .svars
            .keys()
            .filter(|k| k.starts_with("Perpetual"))
            .cloned()
            .collect();
        for key in perpetual_keys {
            ctx.game.card_mut(source).svars.remove(&key);
        }
    }
}
