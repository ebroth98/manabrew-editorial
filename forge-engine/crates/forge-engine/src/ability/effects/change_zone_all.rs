use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, parse_zone_type, EffectContext};
use crate::ids::{CardId, PlayerId};
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    params: &BTreeMap<String, String>,
    entry: &StackEntry,
) {
    let origin_str = params.get("Origin").map(|s| s.as_str()).unwrap_or("Battlefield");
    let destination_str = params.get("Destination").map(|s| s.as_str()).unwrap_or("Graveyard");
    let valid_cards_filter = params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Card".to_string());
    let tapped = params
        .get("Tapped")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    if let (Some(dest_zone), Some(origin_zone)) =
        (parse_zone_type(destination_str), parse_zone_type(origin_str))
    {
        let player_ids = ctx.game.player_order.clone();
        let mut to_move: Vec<(CardId, PlayerId)> = Vec::new();

        for &pid in &player_ids {
            let zone_cards = ctx.game.cards_in_zone(origin_zone, pid).to_vec();
            for cid in zone_cards {
                if matches_change_type(ctx.game.card(cid), &valid_cards_filter) {
                    let dest_owner = if dest_zone == ZoneType::Battlefield {
                        entry.controller
                    } else {
                        ctx.game.card(cid).owner
                    };
                    to_move.push((cid, dest_owner));
                }
            }
        }

        for (card_id, dest_owner) in to_move {
            if ctx.game.card(card_id).zone != origin_zone {
                continue; // already moved
            }
            let old_zone = ctx.game.card(card_id).zone;
            ctx.game.move_card(card_id, dest_zone, dest_owner);
            if dest_zone == ZoneType::Battlefield {
                if tapped {
                    ctx.game.tap(card_id);
                }
                ctx.trigger_handler
                    .register_active_trigger(ctx.game, card_id);
            }
            emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, dest_zone);
        }
    }
}
