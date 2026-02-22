use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            let owner = ctx.game.card(target_card).owner;
            ctx.game.move_card(target_card, ZoneType::Graveyard, owner);
        }
    }
}
