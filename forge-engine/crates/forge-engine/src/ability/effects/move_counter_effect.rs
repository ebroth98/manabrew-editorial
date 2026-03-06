use forge_foundation::ZoneType;

use super::{parse_counter_type, resolve_numeric_svar, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// `SP$ MoveCounter` — move counters from one permanent to another.
///
/// Mirrors Java's `CountersMoveEffect.java`.
/// - `CounterType$` — type of counter to move (default P1P1).
/// - `CounterNum$` — number of counters to move (default 1).
/// - `Source$` — where to take counters from (default Self/source card).
/// - `Defined$` — where to put counters (default Targeted).
///
/// # Card script examples
/// ```text
/// A:SP$ MoveCounter | CounterType$ P1P1 | CounterNum$ 1 | Source$ Self | Defined$ Targeted
/// A:SP$ MoveCounter | CounterType$ CHARGE | CounterNum$ 2
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let counter_type = sa
        .params
        .get("CounterType")
        .map(|s| parse_counter_type(s))
        .unwrap_or(crate::card::CounterType::P1P1);
    let count = resolve_numeric_svar(ctx.game, sa, "CounterNum", 1);
    if count <= 0 {
        return;
    }

    // Determine source of counters
    let from_id = match sa.params.get("Source").map(|s| s.as_str()) {
        Some("Targeted") => sa.target_chosen.target_card,
        Some("ParentTarget") => ctx.parent_target_card,
        _ => sa.source, // Default: Self
    };

    // Determine destination for counters
    let to_id = sa.target_chosen.target_card.or_else(|| {
        match sa.params.get("Defined").map(|s| s.as_str()) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => None,
        }
    });

    let from = match from_id {
        Some(id) if ctx.game.card(id).zone == ZoneType::Battlefield => id,
        _ => return,
    };
    let to = match to_id {
        Some(id) if ctx.game.card(id).zone == ZoneType::Battlefield => id,
        _ => return,
    };

    if from == to {
        return;
    }

    // Calculate how many we can actually move
    let available = ctx.game.card(from).counter_count(&counter_type);
    let actual = count.min(available);
    if actual <= 0 {
        return;
    }

    // Remove from source
    ctx.game
        .card_mut(from)
        .remove_counter(&counter_type, actual);
    ctx.trigger_handler.run_trigger(
        TriggerType::CounterRemoved,
        RunParams {
            card: Some(from),
            counter_type: Some(format!("{:?}", counter_type)),
            counter_amount: Some(actual),
            ..Default::default()
        },
        false,
    );

    // Add to destination
    if crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
        &ctx.game.cards,
        ctx.game.card(to),
        &counter_type,
    ) {
        return;
    }
    let add_amount = if let Some(max) =
        crate::staticability::static_ability_max_counter::max_counter(
            &ctx.game.cards,
            ctx.game.card(to),
            &counter_type,
        ) {
        (max - ctx.game.card(to).counter_count(&counter_type)).clamp(0, actual)
    } else {
        actual
    };
    if add_amount <= 0 {
        return;
    }
    ctx.game.card_mut(to).add_counter(&counter_type, add_amount);
    ctx.trigger_handler.run_trigger(
        TriggerType::CounterAdded,
        RunParams {
            card: Some(to),
            counter_type: Some(format!("{:?}", counter_type)),
            counter_amount: Some(add_amount),
            ..Default::default()
        },
        false,
    );
}
