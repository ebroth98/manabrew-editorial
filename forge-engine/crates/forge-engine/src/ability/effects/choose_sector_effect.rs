//! ChooseSector — choose a sector (Unfinity attraction board).
//! Ported from Java's ChooseSectorEffect: stores chosen sector on host card.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(source) = sa.source {
        // Auto-choose sector 1 (in full implementation, agent would choose)
        let sector = ctx.rng.next_int(6) + 1;
        ctx.game
            .card_mut(source)
            .set_s_var("ChosenSector", format!("Number${}", sector));
    }
}
