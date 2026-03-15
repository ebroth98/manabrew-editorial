//! Untap the source permanent as a cost. Mirrors Java's `CostUntap`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Pay the untap cost by untapping the source.
/// Mirrors Java's `CostUntap.payAsDecided()`.
/// NOTE: Trigger firing (UntapAll) is handled by the caller.
pub fn pay_as_decided(game: &mut GameState, source: CardId) -> bool {
    game.untap(source);
    true
}

/// Refund the untap cost by tapping the source.
/// Mirrors Java's `CostUntap.refund()`.
pub fn refund(game: &mut GameState, source: CardId) {
    game.tap(source);
}
