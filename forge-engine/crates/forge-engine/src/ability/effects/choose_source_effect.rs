use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ ChooseSource` — the activating player chooses a source (permanent/spell).
/// Stores the result for subsequent effects.
///
/// Mirrors Java's `ChooseSourceEffect.java`.
/// - `Choices$` — filter for valid sources (default: Permanent on battlefield).
///
/// # Card script examples
/// ```text
/// A:SP$ ChooseSource | Choices$ Permanent
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let choices_filter = sa
        .params
        .get("Choices")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Permanent".to_string());

    let player_ids = ctx.game.player_order.clone();
    let mut valid: Vec<CardId> = Vec::new();

    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &choices_filter, controller) {
                valid.push(cid);
            }
        }
    }

    if valid.is_empty() {
        return;
    }

    let chosen = ctx.agents[controller.index()].choose_cards_for_effect(controller, &valid, 1, 1);

    if let Some(&chosen_id) = chosen.first() {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).add_chosen_card(chosen_id);
        }
    }
}
