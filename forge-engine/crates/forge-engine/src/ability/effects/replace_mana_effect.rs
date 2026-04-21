//! ReplaceMana effect — replace mana production with different mana.
//!
//! Ported from Java's `ReplaceManaEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ReplaceManaEffect` class extending `SpellAbilityEffect`.
pub struct ReplaceManaEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for ReplaceManaEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Mana replacement is handled by the mana system's replacement handler.
    if let Some(source_id) = sa.source {
        if let Some(val) = sa.params.get(crate::parsing::keys::MANA_REPLACEMENT) {
            ctx.game
                .card_mut(source_id)
                .set_s_var("ManaReplacement", val);
        }
    }
    }
}
