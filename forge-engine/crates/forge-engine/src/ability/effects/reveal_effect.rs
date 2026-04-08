use forge_foundation::ZoneType;

use super::{parse_param, resolve_defined_player, EffectContext};
use crate::agent::GameLogEvent;
use crate::spellability::SpellAbility;

/// Mirrors Java's `RevealEffect.java`.
///
/// `SP$ Reveal | Defined$ You | NumCards$ N`
/// The target player reveals cards from their hand.
/// In the engine, reveal is informational — we notify all agents.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1) as usize;

    let target = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    let hand = ctx.game.cards_in_zone(ZoneType::Hand, target).to_vec();
    if hand.is_empty() {
        return;
    }

    let count = num.min(hand.len());
    let revealed = &hand[hand.len() - count..];

    // Notify all agents of the revealed cards.
    for agent in ctx.agents.iter_mut() {
        for &id in revealed {
            let name = ctx.game.card(id).card_name.clone();
            agent.notify(crate::agent::notification::GameNotification::Event(
                GameLogEvent::rule(format!("Revealed: {}", name)).with_card(id),
            ));
        }
    }
}
