//! ControlExchangeVariant effect — variant of control exchange.
//!
//! Mirrors Java's `ControlExchangeVariantEffect.java`.
//! Exchanges cards controlled between two players, where the activating
//! player chooses which cards to exchange.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve exchange of controlled cards between two target players.
/// Currently delegates to the standard control exchange logic.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // In Java this allows choosing specific cards from each player to swap.
    // Delegates to the existing control exchange/gain variant handler for now.
    super::control_gain_variant_effect::resolve(ctx, sa);
}
