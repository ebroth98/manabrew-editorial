//! Roll dice as a cost. Mirrors Java's `CostRollDice`.
//!
//! Java's `CostRollDice.payAsDecided()` calls `RollDiceEffect.rollDiceForPlayer()`.
//! In Rust, dice rolling + trigger firing is handled by the caller since it
//! requires RNG and trigger handler access.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled inline in game_action.rs because it requires
// RNG (game_rng) and trigger handler access for RolledDie/RolledDieOnce triggers.
// See game_action.rs CostPart::RollDice match arm.
