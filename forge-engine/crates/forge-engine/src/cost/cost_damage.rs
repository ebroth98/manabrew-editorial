//! Deal damage to self as a cost. Mirrors Java's `CostDamage`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay by dealing damage to the player.
/// Mirrors Java's `CostDamage.payAsDecided()` which creates a CardDamageMap
/// and calls `game.getAction().dealDamage()`.
/// NOTE: Trigger firing (DamageDone) must be handled by the caller.
pub fn pay_as_decided(game: &mut GameState, player: PlayerId, amount: i32) -> bool {
    game.deal_damage_to_player(player, amount);
    amount > 0
}
