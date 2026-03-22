//! SpellApiBased — spell abilities backed by an API type.
//!
//! Mirrors Java's `SpellApiBased.java`.
//! A spell (as opposed to an activated ability) whose resolution is
//! dispatched through the effect system based on its `ApiType`.

use crate::spellability::SpellAbility;

/// Marker trait for API-based spell abilities.
///
/// In Java, `SpellApiBased extends Spell` and holds a reference to
/// `SpellAbilityEffect`. In Rust the dispatch is centralized in
/// `effect_dispatch!`, so this provides structural parity.
pub trait SpellApiBased {
    /// The API type string (e.g. "DealDamage", "GainLife").
    fn api_type(&self) -> &str;

    /// Whether this spell is intrinsic to its card.
    fn is_intrinsic(&self) -> bool {
        true
    }

    /// Resolve this spell by dispatching to the effect system.
    fn resolve(&self, sa: &SpellAbility);
}
