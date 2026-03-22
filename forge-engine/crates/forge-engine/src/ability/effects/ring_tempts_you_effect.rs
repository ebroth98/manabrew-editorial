//! RingTempts effect — The Ring tempts you (Lord of the Rings mechanic).
//!
//! Ported from Java's `RingTemptsYouEffect.java`.
//! The Ring tempts you: Your Ring-bearer gains the next level of abilities.
//! If you don't have a Ring-bearer, choose a creature you control.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    for pid in players {
        ring_tempts(ctx, sa, pid);
    }
}

fn ring_tempts(ctx: &mut EffectContext, _sa: &SpellAbility, player: PlayerId) {
    // Increment ring level (max 4)
    let current_level = ctx.game.player(player).ring_level;
    if current_level < 4 {
        ctx.game.player_mut(player).ring_level = current_level + 1;
    }

    // If no ring-bearer, choose one
    let has_bearer = ctx.game.player(player).ring_bearer.is_some();
    if !has_bearer {
        let creatures: Vec<CardId> = ctx.game.cards.iter()
            .filter(|c| {
                c.zone == ZoneType::Battlefield
                    && c.controller == player
                    && c.type_line.core_types.iter().any(|ct| matches!(ct, forge_foundation::CoreType::Creature))
            })
            .map(|c| c.id)
            .collect();

        if !creatures.is_empty() {
            ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
            if let Some(chosen) = ctx.agents[player.index()].choose_single_card_for_zone_change(
                player, &creatures, "Choose your Ring-bearer", false,
            ) {
                ctx.game.player_mut(player).ring_bearer = Some(chosen);
            }
        }
    }
}
