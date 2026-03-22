//! ManifestBaseEffect — abstract base for manifest variants.
//!
//! Mirrors Java's `ManifestBaseEffect.java`.
//! Provides shared logic for `ManifestEffect`, `ManifestDreadEffect`,
//! and `CloakEffect` that puts cards onto the battlefield face-down
//! as 2/2 creatures.

use crate::spellability::SpellAbility;

use super::EffectContext;

/// Common manifest parameters parsed from a spell ability.
pub struct ManifestParams {
    /// Number of cards to manifest.
    pub amount: usize,
    /// Whether the manifested cards come from the library.
    pub from_library: bool,
}

/// Parse common manifest parameters from a spell ability.
pub fn parse_manifest_params(ctx: &EffectContext, sa: &SpellAbility) -> ManifestParams {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(1) as usize;
    let from_library = sa
        .params
        .get("Defined")
        .map_or(true, |d| d == "TopOfLibrary");
    ManifestParams {
        amount,
        from_library,
    }
}

/// Get the default message for manifest choice prompts.
pub fn default_manifest_message() -> &'static str {
    "Choose a card to manifest"
}

/// Get the default message for manifest dread choice prompts.
pub fn default_manifest_dread_message() -> &'static str {
    "Choose a card to manifest dread"
}

/// Get the default message for cloak choice prompts.
pub fn default_cloak_message() -> &'static str {
    "Choose a card to cloak"
}
