use super::EffectContext;
use crate::spellability::SpellAbility;

/// Mirrors Java's `CleanupEffect.java`.
///
/// `DB$ Cleanup | ClearRemembered$ True`
///
/// Clears remembered cards and CMC values from the source card.
/// Used at the end of transform trigger chains (e.g. Delver of Secrets).
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `CleanupEffect` class extending `SpellAbilityEffect`.
pub struct CleanupEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for CleanupEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let clear_remembered = sa
        .params
        .get("ClearRemembered")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    if clear_remembered {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).clear_remembered();
        }
    }
    }
}
