//! roll_planar_dice effect — ported from Java.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::niche_effects::resolve_roll_planar_dice(ctx, sa);
}
