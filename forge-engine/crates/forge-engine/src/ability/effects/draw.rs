use std::collections::BTreeMap;

use super::{parse_param, resolve_defined_player, EffectContext};
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    params: &BTreeMap<String, String>,
    entry: &StackEntry,
    ability: &str,
) {
    let num = parse_param(ability, "NumCards$ ").unwrap_or(1);
    let target = params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, entry.controller, ctx.game))
        .unwrap_or(entry.controller);
    ctx.game.draw_cards(target, num as usize);
}
