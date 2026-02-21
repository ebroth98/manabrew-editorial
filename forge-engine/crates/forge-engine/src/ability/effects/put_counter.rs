use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::{parse_counter_type, parse_param, EffectContext};
use crate::card::CounterType;
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    params: &BTreeMap<String, String>,
    entry: &StackEntry,
    ability: &str,
) {
    let counter_type = params
        .get("CounterType")
        .map(|s| parse_counter_type(s))
        .unwrap_or(CounterType::P1P1);
    let count = parse_param(ability, "CounterNum$ ").unwrap_or(1);

    // Default target: the source card
    if let Some(card_id) = entry.source {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.card_mut(card_id).add_counter(counter_type, count);
        }
    }
}
