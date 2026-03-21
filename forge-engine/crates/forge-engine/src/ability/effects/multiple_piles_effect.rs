//! multiple_piles effect — ported from Java.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::niche_effects::resolve_multiple_piles(ctx, sa);
}
