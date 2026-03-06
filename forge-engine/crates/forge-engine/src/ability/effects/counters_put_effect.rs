use forge_foundation::ZoneType;

use super::{
    parse_counter_type, parse_param, resolve_defined_player, resolve_numeric_svar, EffectContext,
};
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::replacement::handler::{apply_replacements, ReplacementEvent};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let counter_type_str = sa
        .params
        .get("CounterType")
        .map(|s| s.as_str())
        .unwrap_or("P1P1");
    let counter_type = parse_counter_type(counter_type_str);
    // Support SVar references for CounterNum (e.g. Count$Kicked.4.0 for kicker cards)
    let count = parse_param(&sa.ability_text, "CounterNum$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "CounterNum", 1));

    // Resolve the controller of this ability (for Defined$ You etc.)
    let controller = sa
        .source
        .map(|id| ctx.game.card(id).controller)
        .unwrap_or_else(|| ctx.game.player_order[0]);

    // Check for Defined$ — if targeting a player (e.g. Defined$ You for energy),
    // handle player-level counters like ENERGY instead of card counters.
    if let Some(defined) = sa.params.get("Defined") {
        if let Some(target_player) = resolve_defined_player(defined, controller, ctx.game) {
            match counter_type_str.to_uppercase().as_str() {
                "ENERGY" => {
                    ctx.game.player_mut(target_player).energy_counters += count;
                    return;
                }
                _ => {
                    // Other player-level counters (e.g. EXPERIENCE) can be
                    // added here in the future. For now, fall through to
                    // the card path if we somehow arrive here.
                }
            }
        }
    }

    // Resolve target card based on Defined$ parameter.
    let defined = sa
        .params
        .get("Defined")
        .map(|s| s.as_str())
        .unwrap_or("Self");
    let target_id = match defined {
        "TriggeredTarget" | "TriggeredTargetLKICopy" => sa.target_chosen.target_card.or(sa.source),
        "Targeted" => sa.target_chosen.target_card.or(sa.source),
        "Self" | _ => sa.source,
    };

    let Some(card_id) = target_id else { return };
    if ctx.game.card(card_id).zone != ZoneType::Battlefield {
        return;
    }

    if crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
        &ctx.game.cards,
        ctx.game.card(card_id),
        &counter_type,
    ) {
        return;
    }
    if let Some(max) = crate::staticability::static_ability_max_counter::max_counter(
        &ctx.game.cards,
        ctx.game.card(card_id),
        &counter_type,
    ) {
        let current = ctx.game.card(card_id).counter_count(&counter_type);
        if current >= max {
            return;
        }
    }
    // Run AddCounter replacement effects (e.g. Hardened Scales adds extra).
    let mut event = ReplacementEvent::AddCounter {
        target: card_id,
        counter_type: counter_type.clone(),
        count,
    };
    apply_replacements(ctx.game, &mut event);
    let count = if let ReplacementEvent::AddCounter {
        count: final_count, ..
    } = event
    {
        final_count
    } else {
        count
    };
    let cause_player = ctx.game.card(card_id).controller;
    ctx.game.card_mut(card_id).add_counter(&counter_type, count);

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
