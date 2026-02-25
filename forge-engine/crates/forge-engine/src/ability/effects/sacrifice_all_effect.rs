use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let valid_cards_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Creature".to_string());

    let player_ids = ctx.game.player_order.clone();
    let mut to_sacrifice: Vec<CardId> = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_change_type(ctx.game.card(cid), &valid_cards_filter, &[]) {
                to_sacrifice.push(cid);
            }
        }
    }

    for card_id in to_sacrifice {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }
        let controller = ctx.game.card(card_id).controller;
        let owner = ctx.game.card(card_id).owner;
        // Fire Sacrificed trigger
        ctx.trigger_handler.run_trigger(
            TriggerType::Sacrificed,
            RunParams {
                card: Some(card_id),
                player: Some(controller),
                ..Default::default()
            },
            false,
        );
        ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
        emit_zone_trigger(
            ctx.trigger_handler,
            card_id,
            ZoneType::Battlefield,
            ZoneType::Graveyard,
        );
    }
}
