use forge_foundation::ZoneType;

use super::{
    matches_valid_cards, parse_counter_type, parse_param, resolve_numeric_svar, EffectContext,
};
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ PutCounterAll` — put counters on all matching permanents.
///
/// Mirrors Java's `CountersPutAllEffect.java`.
/// - `CounterType$` — type of counter (default P1P1).
/// - `CounterNum$` — number of counters to add (default 1).
/// - `ValidCards$` — filter for which cards receive counters.
/// - `ValidZone$` — zone to search (default Battlefield).
///
/// # Card script examples
/// ```text
/// A:SP$ PutCounterAll | CounterType$ P1P1 | CounterNum$ 1 | ValidCards$ Creature.YouCtrl
/// A:SP$ PutCounterAll | CounterType$ CHARGE | CounterNum$ 2 | ValidCards$ Artifact
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let counter_type = sa
        .params
        .get("CounterType")
        .map(|s| parse_counter_type(s))
        .unwrap_or(CounterType::P1P1);
    let count = parse_param(&sa.ability_text, "CounterNum$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "CounterNum", 1));
    if count == 0 {
        return;
    }

    let valid_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Creature".to_string());
    let zone = sa
        .params
        .get("ValidZone")
        .and_then(|z| super::parse_zone_type(z))
        .unwrap_or(ZoneType::Battlefield);

    let player_ids = ctx.game.player_order.clone();
    let mut targets: Vec<CardId> = Vec::new();

    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(zone, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_filter, sa.activating_player) {
                targets.push(cid);
            }
        }
    }

    for card_id in targets {
        if ctx.game.card(card_id).zone == zone {
            if crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                &ctx.game.cards,
                ctx.game.card(card_id),
                &counter_type,
            ) {
                continue;
            }
            let add_count = if let Some(max) = crate::staticability::static_ability_max_counter::max_counter(
                &ctx.game.cards,
                ctx.game.card(card_id),
                &counter_type,
            ) {
                (max - ctx.game.card(card_id).counter_count(&counter_type)).clamp(0, count)
            } else {
                count
            };
            if add_count <= 0 {
                continue;
            }
            ctx.game.card_mut(card_id).add_counter(&counter_type, add_count);
            ctx.trigger_handler.run_trigger(
                TriggerType::CounterAdded,
                RunParams {
                    card: Some(card_id),
                    counter_type: Some(format!("{:?}", counter_type)),
                    counter_amount: Some(add_count),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
