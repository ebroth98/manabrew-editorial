//! Reveal cards as a cost. Mirrors Java's `CostReveal`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_reveal_cost() in game_action.rs
// because it requires agent interaction for card selection.
// Java's CostReveal.doPayment() calls game.getAction().reveal() which is display-only.

pub const HASH_LKI: &str = "Revealed";
pub const HASH_CARDS: &str = "RevealedCards";
