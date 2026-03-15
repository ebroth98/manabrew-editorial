//! Choose color(s) as a cost. Mirrors Java's `CostChooseColor`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Pay by setting chosen colors on the source card.
/// Mirrors Java's `CostChooseColor.payAsDecided()` →
/// `sa.getHostCard().setChosenColors(colors)`.
pub fn pay_as_decided(game: &mut GameState, source: CardId, colors: &[String]) -> bool {
    game.card_mut(source).chosen_colors = colors.to_vec();
    true
}

/// Refund by clearing chosen colors.
/// Mirrors Java's `CostChooseColor.refund()`.
pub fn refund(game: &mut GameState, source: CardId) {
    game.card_mut(source).chosen_colors.clear();
}
