//! Waterbend as a cost. Mirrors Java's `CostWaterbend` which extends `CostPartMana`.
//!
//! Waterbend N means pay N generic mana, but you can tap your artifacts and creatures
//! to help pay (each tapped = {1}, like convoke + improvise combined).
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided for waterbend requires agent interaction (choose_convoke)
// and mana pool access, so it stays in GameLoop::pay_waterbend_cost() in game_action.rs.
