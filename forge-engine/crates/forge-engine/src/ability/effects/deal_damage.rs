use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::{parse_num_dmg, resolve_defined_player, EffectContext};
use crate::spellability::StackEntry;
use crate::trigger::parse_pipe_params;

pub fn resolve(
    ctx: &mut EffectContext,
    _params: &BTreeMap<String, String>,
    entry: &StackEntry,
    ability: &str,
) {
    let damage = parse_num_dmg(ability);

    // For triggered abilities, resolve Defined$ for target
    let target_player = entry.target_player.or_else(|| {
        let params = parse_pipe_params(ability);
        if let Some(defined) = params.get("Defined") {
            resolve_defined_player(defined, entry.controller, ctx.game)
        } else {
            None
        }
    });

    if let Some(target_player) = target_player {
        ctx.game.deal_damage_to_player(target_player, damage);
    }
    if let Some(target_card) = entry.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.deal_damage_to_card(target_card, damage);
        }
    }
}
