//! Meld effect — exile two named cards, return melded as one creature.
//!
//! Ported 1:1 from Java's `MeldEffect.java`.
//! Meld: Exile two specific named permanents you control and own,
//! then return the primary card to the battlefield transformed (melded)
//! with the secondary card linked to it.

use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(host_card_id) = sa.source else { return };
    let controller = sa.activating_player;

    let prim_name = sa.params.get("Primary").cloned().unwrap_or_default();
    let sec_name = sa.params.get("Secondary").cloned().unwrap_or_default();
    let sec_type = sa.params.get("SecondaryType").cloned().unwrap_or_else(|| "Creature".to_string());

    // Find a permanent you control and own named "Secondary" matching SecondaryType
    let candidates: Vec<CardId> = ctx.game.cards.iter()
        .filter(|c| {
            c.zone == ZoneType::Battlefield
                && c.controller == controller
                && c.owner == controller
                && c.card_name == sec_name
                && super::matches_change_type(c, &sec_type, &[])
        })
        .map(|c| c.id)
        .collect();

    if candidates.is_empty() {
        return;
    }

    // Choose which secondary to meld with (if multiple copies)
    let secondary_id = if candidates.len() == 1 {
        candidates[0]
    } else {
        ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[controller.index()]
            .choose_single_card_for_zone_change(
                controller,
                &candidates,
                "Choose a card to meld",
                false,
            )
            .unwrap_or(candidates[0])
    };

    // Exile both cards
    let cards_to_exile = vec![host_card_id, secondary_id];
    let mut exiled = Vec::new();

    for &card_id in &cards_to_exile {
        // canExiledBy check
        if ctx.game.card(card_id).keywords.iter().any(|k| k.eq_ignore_ascii_case("CantBeExiled")) {
            continue;
        }
        let old_zone = ctx.game.card(card_id).zone;
        ctx.game.move_card(card_id, ZoneType::Exile, controller);
        emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Exile);
        exiled.push(card_id);
    }

    // Both must have been exiled
    if exiled.len() < 2 {
        return;
    }

    let primary = exiled[0];
    let secondary = exiled[1];

    // Verify names are still correct in exile
    if ctx.game.card(primary).card_name != prim_name
        || ctx.game.card(secondary).card_name != sec_name
    {
        return;
    }

    // Verify neither is a token and both are still in exile
    for &c in &[primary, secondary] {
        if ctx.game.card(c).is_token || ctx.game.card(c).zone != ZoneType::Exile {
            return;
        }
    }

    // Transform primary to meld state
    ctx.game.card_mut(primary).is_transformed = true;
    if let Some(ref other) = ctx.game.card(primary).other_part {
        ctx.game.card_mut(primary).card_name = other.name.clone();
    }

    // Link secondary to primary via melded_with
    ctx.game.card_mut(primary).melded_with.push(secondary);

    // Tapped$
    if sa.is_tapped() {
        ctx.game.card_mut(primary).tapped = true;
    }

    // Move primary to battlefield (melded form)
    let old_zone = ctx.game.card(primary).zone;
    ctx.game.move_card(primary, ZoneType::Battlefield, controller);
    ctx.trigger_handler.register_active_trigger(ctx.game, primary);
    emit_zone_trigger(ctx.trigger_handler, primary, old_zone, ZoneType::Battlefield);

    // Attacking$ combat entry
    if sa.param_is_true("Attacking") {
        let defender = ctx.game.opponent_of(controller);
        ctx.game.card_mut(primary).attacking_player = Some(defender);
    }
}
