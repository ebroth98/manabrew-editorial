use super::EffectContext;
use crate::spellability::SpellAbility;

/// Mirrors Java's `CleanupEffect.java`.
///
/// `DB$ Cleanup | ClearRemembered$ True`
///
/// Clears remembered cards and CMC values from the source card.
/// Used at the end of transform trigger chains (e.g. Delver of Secrets).
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let clear_remembered = sa
        .params
        .get("ClearRemembered")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    if clear_remembered {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).remembered_cards.clear();
            ctx.game.card_mut(source_id).remembered_cmc.clear();
        }
    }
}
