use forge_foundation::ZoneType;

use super::{parse_param, parse_zone_type, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

/// Mirrors Java's `LookAtEffect.java`.
///
/// `SP$ LookAt | Defined$ You | NumCards$ N`
/// The activating player looks at cards in a hidden zone (e.g. top of library or opponent's hand)
/// without revealing them to others.
/// In the engine this is informational — we notify only the activating player's agent.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ")
        .or_else(|| parse_param(&sa.ability_text, "ScryNum$ "))
        .unwrap_or(1) as usize;

    let source_zone = sa
        .params
        .get("SourceZone")
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Library);

    let look_player = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    let zone_cards = ctx.game.cards_in_zone(source_zone, look_player).to_vec();
    let count = num.min(zone_cards.len());

    let top = &zone_cards[zone_cards.len() - count..];
    let names: Vec<String> = top.iter().map(|&id| ctx.game.card(id).card_name.clone()).collect();
    let msg = format!(
        "Looking at top {} card(s) of {:?}: [{}]",
        count,
        source_zone,
        names.join(", ")
    );
    // Only the activating player can see these.
    ctx.agents[sa.activating_player.index()].notify(&msg);
}
