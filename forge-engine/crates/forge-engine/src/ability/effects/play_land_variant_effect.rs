//! PlayLandVariant effect — variant play land.
//!
//! Mirrors Java's `PlayLandVariantEffect.java`.
//! Allows a card to be played as a copy of a basic land matching
//! one of its colors.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve play land variant.
/// In Java this clones a random basic land matching the source's colors
/// and plays it. Currently a stub for structural parity.
pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // PlayLandVariant is a niche effect used by cards like Dryad of the Ilysian Grove.
    // Full implementation requires the card database for land lookup.
    let err = crate::ability::IllegalAbilityException::new(
        "PlayLandVariant effect not yet fully implemented",
    );
    eprintln!("{}", err);
}
