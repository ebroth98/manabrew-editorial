//! Have an opponent gain life as a cost. Mirrors Java's `CostGainLife`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay by having the opponent gain life.
/// Mirrors Java's `CostGainLife.payAsDecided()`.
pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    let opponent = game.opponent_of(player);
    game.player_mut(opponent).gain_life(amount);
    true
}
