use forge_foundation::ZoneType;

use super::{emit_zone_trigger_with_lki_counters, matches_change_type, EffectContext};
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
        // Capture +1/+1 counter count before move (for Modular death triggers)
        let lki_p1p1 = *ctx
            .game
            .card(card_id)
            .counters
            .get(&crate::card::CounterType::P1P1)
            .unwrap_or(&0);
        // Clear temporary Animate triggers before firing events (CR 400.7).
        {
            let card = ctx.game.card_mut(card_id);
            let pt = card.pump_trigger_count;
            if pt > 0 {
                let new_len = card.triggers.len().saturating_sub(pt);
                card.triggers.truncate(new_len);
                card.pump_trigger_count = 0;
            }
        }
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
        emit_zone_trigger_with_lki_counters(
            ctx.trigger_handler,
            card_id,
            ZoneType::Battlefield,
            ZoneType::Graveyard,
            lki_p1p1,
        );
        ctx.trigger_handler.flush_waiting_triggers(ctx.game);
        ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
    }
}
