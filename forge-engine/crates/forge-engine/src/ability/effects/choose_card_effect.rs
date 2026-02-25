use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_zone_type, EffectContext};
use crate::spellability::SpellAbility;

/// `SP$ ChooseCard` — player chooses card(s) from a filtered set in a zone.
///
/// Mirrors Java's `ChooseCardEffect.java`.
///
/// # Params
/// - `Amount` — how many cards to choose (default 1)
/// - `ChoiceZone` — zone to choose from (default Battlefield)
/// - `Choices` — ValidCards filter for eligible cards
/// - `RememberChosen` — if "True", add chosen to source's remembered_cards
///
/// Stores the chosen card(s) on the source card's `chosen_cards`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let controller = sa.activating_player;

    let amount: usize = sa
        .params
        .get("Amount")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let zone = sa
        .params
        .get("ChoiceZone")
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Battlefield);

    let filter = sa
        .params
        .get("Choices")
        .cloned()
        .unwrap_or_else(|| "Card".to_string());

    let remember = sa
        .params
        .get("RememberChosen")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    // Collect valid cards in zone matching filter
    let mut valid = Vec::new();
    for &pid in &ctx.game.player_order.clone() {
        let zone_cards = ctx.game.cards_in_zone(zone, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &filter, controller) {
                valid.push(cid);
            }
        }
    }

    if valid.is_empty() {
        return;
    }

    // Ask the controlling player to choose
    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
    let chosen =
        ctx.agents[controller.index()].choose_cards_for_effect(controller, &valid, 1, amount);

    // Store on source card
    ctx.game.card_mut(source_id).chosen_cards = chosen.clone();

    // Optionally remember
    if remember {
        for &cid in &chosen {
            ctx.game.card_mut(source_id).add_remembered_card(cid);
        }
    }
}
