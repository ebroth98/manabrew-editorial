//! Pay mana shards as a cost. Mirrors Java's `CostPayShards`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    game.player_mut(player).mana_shards -= amount;
    true
}

pub fn refund(game: &mut GameState, player: PlayerId, amount: i32) {
    game.player_mut(player).mana_shards += amount;
}
