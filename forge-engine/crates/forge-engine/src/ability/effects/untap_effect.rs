use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ Untap` — untap a single permanent.
///
/// Mirrors Java's `UntapEffect.java`.
///
/// # Card script examples
/// ```text
/// DB$ Untap | Defined$ Self
/// DB$ Untap | Defined$ ParentTarget
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let target_card = resolve_untap_target(ctx, sa);

    if let Some(card_id) = target_card {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.untap(card_id);
            // Fire Untaps trigger
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::Untaps,
                crate::event::RunParams {
                    card: Some(card_id),
                    ..Default::default()
                },
                false,
            );
        }
    }
}

/// Resolve the target card for untap: explicit target, Defined$ Self, or Defined$ ParentTarget.
fn resolve_untap_target(ctx: &EffectContext, sa: &SpellAbility) -> Option<CardId> {
    sa.target_chosen.target_card.or_else(|| {
        match sa.params.get("Defined").map(|s| s.as_str()) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => None,
        }
    })
}
