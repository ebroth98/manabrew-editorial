//! Enlist a creature as a cost. Mirrors Java's `CostEnlist`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_enlist_cost() in game_action.rs
// because it requires agent interaction, tapping, power transfer, and trigger firing (Enlisted).

pub const HASH_LKI: &str = "Enlisted";
pub const HASH_CARDS: &str = "EnlistedCards";
