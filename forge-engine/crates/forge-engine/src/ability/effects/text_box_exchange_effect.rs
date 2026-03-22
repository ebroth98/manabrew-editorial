//! TextBoxExchange effect — swap text boxes of two cards.
//!
//! Ported from Java's `TextBoxExchangeEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;
use forge_foundation::ZoneType;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source = sa.source;
    let target = sa.target_chosen.target_card;

    let (Some(card1), Some(card2)) = (source, target) else { return };
    if ctx.game.card(card1).zone != ZoneType::Battlefield { return; }
    if ctx.game.card(card2).zone != ZoneType::Battlefield { return; }

    // Swap abilities text
    let abilities1 = ctx.game.card(card1).abilities.clone();
    let abilities2 = ctx.game.card(card2).abilities.clone();
    ctx.game.card_mut(card1).set_abilities(abilities2);
    ctx.game.card_mut(card2).set_abilities(abilities1);

    // Re-register triggers
    ctx.trigger_handler.register_active_trigger(ctx.game, card1);
    ctx.trigger_handler.register_active_trigger(ctx.game, card2);
}
