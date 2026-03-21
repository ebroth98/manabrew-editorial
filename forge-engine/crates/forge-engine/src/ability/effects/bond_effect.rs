//! Bond effect — partner/bond mechanic for pairing creatures.
//!
//! Ported from Java's `BondEffect.java`.
//! Bond: Pair two creatures together (Soulbond).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else { return };
    let controller = sa.activating_player;

    if ctx.game.card(source_id).zone != ZoneType::Battlefield { return; }

    // Find an unpaired creature to bond with
    let candidates: Vec<CardId> = ctx.game.cards.iter()
        .filter(|c| {
            c.zone == ZoneType::Battlefield
                && c.controller == controller
                && c.id != source_id
                && c.type_line.core_types.iter().any(|ct| matches!(ct, forge_foundation::CoreType::Creature))
                && c.paired_with.is_none()
        })
        .map(|c| c.id)
        .collect();

    if candidates.is_empty() { return; }

    // Optional — player may decline
    if sa.is_optional() {
        ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
        if !ctx.agents[controller.index()].confirm_action(
            controller, Some("Bond"), "Pair with a creature?", &[], None, None,
        ) { return; }
    }

    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
    if let Some(partner) = ctx.agents[controller.index()].choose_single_card_for_zone_change(
        controller, &candidates, "Choose a creature to pair with", false,
    ) {
        ctx.game.card_mut(source_id).paired_with = Some(partner);
        ctx.game.card_mut(partner).paired_with = Some(source_id);
    }
}
