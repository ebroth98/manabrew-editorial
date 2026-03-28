//! RemoveFromGame — exile (old terminology).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(target) = sa.target_chosen.target_card.or(sa.source) {
        let old = ctx.game.card(target).zone;
        let owner = ctx.game.card(target).owner;
        ctx.move_card(target, ZoneType::Exile, owner);
        super::emit_zone_trigger(ctx.trigger_handler, target, old, ZoneType::Exile);
    }
}
