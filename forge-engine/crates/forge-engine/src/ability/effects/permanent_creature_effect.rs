//! PermanentCreature — move host card to battlefield as a creature permanent.
//! Ported from Java's PermanentCreatureEffect.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    super::permanent_effect::resolve_permanent_common(ctx, sa);
}
