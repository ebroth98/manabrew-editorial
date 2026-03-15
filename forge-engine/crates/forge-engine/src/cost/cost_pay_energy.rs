//! Pay energy counters as a cost. Mirrors Java's `CostPayEnergy`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay by removing energy counters.
/// Mirrors Java's `CostPayEnergy.payAsDecided()` → `player.payEnergy(amount)`.
pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    game.player_mut(player).energy_counters -= amount;
    true
}

/// Refund energy payment.
/// Mirrors Java's `CostPayEnergy.refund()` → `source.getController().loseEnergy(-amount)`.
pub fn refund(game: &mut GameState, player: PlayerId, amount: i32) {
    game.player_mut(player).energy_counters += amount;
}
