use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    _params: &BTreeMap<String, String>,
    entry: &StackEntry,
) {
    if let Some(target_card) = entry.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            let owner = ctx.game.card(target_card).owner;
            ctx.game.move_card(target_card, ZoneType::Graveyard, owner);
        }
    }
}
