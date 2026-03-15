//! Forage as a cost. Mirrors Java's `CostForage`.
//!
//! Forage: exile 3 cards from your graveyard, or sacrifice a Food.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_forage_cost() in game_action.rs
// because it requires agent interaction (choose GY cards or Food) and trigger firing (Forage).

pub const HASH_LKI: &str = "Foraged";
pub const HASH_CARDS: &str = "ForagedCards";
