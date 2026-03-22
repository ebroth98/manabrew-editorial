//! ChangeZoneResolve effect — resolves accumulated zone changes.
//!
//! Mirrors Java's `ChangeZoneResolveEffect.java`.
//! Triggers zone-change events for all cards that moved during
//! a batched zone-change operation.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve accumulated zone change triggers.
/// In practice this is handled by the zone-change bookkeeping in the engine;
/// this effect serves as the explicit resolution point matching Java's pattern.
pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {
    // In the Rust engine, zone-change triggers are fired inline during
    // change_zone operations. This stub maintains structural parity with
    // Java's ChangeZoneResolveEffect which clears and fires the
    // CardZoneTable accumulated during multi-card zone changes.
}
