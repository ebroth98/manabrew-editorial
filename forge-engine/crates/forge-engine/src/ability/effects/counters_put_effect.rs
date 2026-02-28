use forge_foundation::ZoneType;

use super::{parse_counter_type, parse_param, resolve_numeric_svar, EffectContext};
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let counter_type = sa
        .params
        .get("CounterType")
        .map(|s| parse_counter_type(s))
        .unwrap_or(CounterType::P1P1);
    // Support SVar references for CounterNum (e.g. Count$Kicked.4.0 for kicker cards)
    let count = parse_param(&sa.ability_text, "CounterNum$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "CounterNum", 1));

    // Default target: the source card
    if let Some(card_id) = sa.source {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            if crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                &ctx.game.cards,
                ctx.game.card(card_id),
                counter_type,
            ) {
                return;
            }
            if let Some(max) = crate::staticability::static_ability_max_counter::max_counter(
                &ctx.game.cards,
                ctx.game.card(card_id),
                counter_type,
            ) {
                let current = ctx.game.card(card_id).counter_count(counter_type);
                if current >= max {
                    return;
                }
            }
            let cause_player = ctx.game.card(card_id).controller;

            ctx.game.card_mut(card_id).add_counter(counter_type, count);

            // Fire CounterAdded trigger
            ctx.trigger_handler.run_trigger(
                TriggerType::CounterAdded,
                RunParams {
                    card: Some(card_id),
                    counter_type: Some(format!("{:?}", counter_type)),
                    counter_amount: Some(count),
                    cause_player: Some(cause_player),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
