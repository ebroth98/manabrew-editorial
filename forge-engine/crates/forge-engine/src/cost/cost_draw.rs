//! Draw cards as a cost. Mirrors Java's `CostDraw`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay by drawing cards.
/// Mirrors Java's `CostDraw.payAsDecided()`.
pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    for _ in 0..amount {
        game.draw_card(player);
    }
    true
}
