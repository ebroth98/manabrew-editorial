//! Pay life as a cost. Mirrors Java's `CostPayLife`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay the life cost.
/// Mirrors Java's `CostPayLife.payAsDecided()` → `player.payLife(amount, ability, effect)`.
/// NOTE: Trigger firing (LifeLost) must be handled by the caller.
pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    if amount <= 0 {
        return true;
    }
    game.player_mut(player).life -= amount;
    // TODO: Fire LifeLost trigger — Java's player.payLife() fires LoseLife trigger.
    // Currently handled by GameLoop::pay_life_cost() which also checks cant_pay_life.
    true
}

/// No refund for life payment.
/// Java's CostPayLife does not override refund().
pub fn refund(_game: &mut GameState, _player: PlayerId, _amount: i32) {
    // Life payment is not refundable in Java.
    // The transactional snapshot handles rollback.
}
