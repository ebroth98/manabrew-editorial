use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::{parse_param, EffectContext};
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    _params: &BTreeMap<String, String>,
    entry: &StackEntry,
    ability: &str,
) {
    let att_bonus = parse_param(ability, "NumAtt$ ").unwrap_or(0);
    let def_bonus = parse_param(ability, "NumDef$ ").unwrap_or(0);

    if let Some(target_card) = entry.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target_card).power_modifier += att_bonus;
            ctx.game.card_mut(target_card).toughness_modifier += def_bonus;
        }
    }
}
