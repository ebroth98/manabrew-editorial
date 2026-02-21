use forge_foundation::ZoneType;

use super::{resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

/// Mirrors Java's `RevealHandEffect.java`.
///
/// `SP$ RevealHand | Defined$ Player`
/// The target player reveals their entire hand to all other players.
/// In the engine this is informational — we notify all agents.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
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

    let names: Vec<String> = hand.iter().map(|&id| ctx.game.card(id).card_name.clone()).collect();
    let msg = format!(
        "Player {} reveals their hand: [{}]",
        target.0,
        names.join(", ")
    );
    for agent in ctx.agents.iter_mut() {
        agent.notify(&msg);
    }
}
