//! Regeneration — set up a regeneration shield (older mechanic).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(target) = sa.target_chosen.target_card.or(sa.source) {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).regeneration_shields += 1;
        }
    }
}
