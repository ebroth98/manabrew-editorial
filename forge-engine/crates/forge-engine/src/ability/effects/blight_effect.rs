//! Blight — mark a permanent with a blight counter or effect.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(target) = sa.target_chosen.target_card {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            let ct = super::parse_counter_type("BLIGHT");
            ctx.game.card_mut(target).add_counter(&ct, 1);
        }
    }
}
