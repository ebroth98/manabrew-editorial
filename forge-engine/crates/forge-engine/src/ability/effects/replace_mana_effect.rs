//! ReplaceMana effect — replace mana production with different mana.
//!
//! Ported from Java's `ReplaceManaEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Mana replacement is handled by the mana system's replacement handler.
    if let Some(source_id) = sa.source {
        if let Some(val) = sa.params.get(crate::parsing::keys::MANA_REPLACEMENT) {
            ctx.game.card_mut(source_id).svars.insert(
                "ManaReplacement".to_string(), val.to_string(),
            );
        }
    }
}
