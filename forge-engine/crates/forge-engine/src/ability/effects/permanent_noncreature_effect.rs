//! PermanentNoncreature — move host card to battlefield as a noncreature permanent.
//! Ported from Java's PermanentNoncreatureEffect.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::permanent_effect::resolve_permanent_common(ctx, sa);
}
