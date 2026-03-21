//! blight effect — ported from Java.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::niche_effects::resolve_blight(ctx, sa);
}
