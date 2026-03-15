//! Tap the source permanent as a cost. Mirrors Java's `CostTap`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Pay the tap cost by tapping the source.
/// Mirrors Java's `CostTap.payAsDecided()`.
/// NOTE: Trigger firing (TapAll) is handled by the caller (GameLoop) since
/// it requires access to the trigger handler which is not available here.
pub fn pay_as_decided(game: &mut GameState, source: CardId) -> bool {
    game.tap(source);
    true
}

/// Refund the tap cost by untapping the source.
/// Mirrors Java's `CostTap.refund()`.
pub fn refund(game: &mut GameState, source: CardId) {
    game.untap(source);
}
