//! ReplaceDamage effect — replace damage with another effect.
//!
//! Ported from Java's `ReplaceDamageEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Damage replacement is handled by the replacement handler system.
    // This effect registers or configures the replacement.
    if let Some(source_id) = sa.source {
        if let Some(val) = sa.params.get("DamageAmount") {
            ctx.game.card_mut(source_id).svars.insert(
                "ReplaceDamageAmount".to_string(), val.clone(),
            );
        }
    }
}
