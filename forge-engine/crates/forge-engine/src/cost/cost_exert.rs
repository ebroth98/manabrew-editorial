//! Exert permanents as a cost. Mirrors Java's `CostExert`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_exert_cost() in game_action.rs
// because it requires agent interaction and trigger firing (Exerted).

pub const HASH_LKI: &str = "Exerted";
pub const HASH_CARDS: &str = "ExertedCards";
