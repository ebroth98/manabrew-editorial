//! Camouflage — old-school combat concealment.
//! Ported from Java's CamouflageEffect: the attacking player divides their
//! creatures into face-down piles, then combat blockers are randomly assigned.
//! In digital: we randomize the blocker assignments since the physical
//! "face-down piles" mechanic can't be faithfully reproduced.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Camouflage works by randomizing blocker assignments.
    // In our engine, combat blocking is handled by the combat system.
    // We mark the source creature so combat resolution knows to randomize.
    if let Some(source) = sa.source {
        if ctx.game.card(source).zone == ZoneType::Battlefield {
            ctx.game
                .card_mut(source)
                .svars
                .insert("Camouflage".to_string(), "True".to_string());
        }
    }
}
