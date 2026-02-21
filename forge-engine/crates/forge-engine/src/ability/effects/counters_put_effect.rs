use forge_foundation::ZoneType;

use super::{parse_counter_type, parse_param, EffectContext};
use crate::card::CounterType;
use crate::spellability::SpellAbility;

pub fn resolve(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
) {
    let counter_type = sa
        .params
        .get("CounterType")
        .map(|s| parse_counter_type(s))
        .unwrap_or(CounterType::P1P1);
    let count = parse_param(&sa.ability_text, "CounterNum$ ").unwrap_or(1);

    // Default target: the source card
    if let Some(card_id) = sa.source {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.card_mut(card_id).add_counter(counter_type, count);
        }
    }
}
