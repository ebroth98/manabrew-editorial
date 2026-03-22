//! DamageResolve — resolve accumulated damage from a damage map.
//! Ported from Java's DamageResolveEffect.
//! In our engine, damage is applied directly, so this is a no-op.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // In Java, this processes a CardDamageMap accumulated by prior effects.
    // In our engine, damage is applied directly in deal_damage effects.
    // This is a synchronization point — no additional work needed since
    // damage is already applied when DealDamage resolves.
}
