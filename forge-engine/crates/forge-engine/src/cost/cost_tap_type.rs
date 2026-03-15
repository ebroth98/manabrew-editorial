//! Tap other permanents of a type as a cost. Mirrors Java's `CostTapType`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Pay by tapping the selected cards.
/// Cards are passed in (already selected by agent).
/// Mirrors Java's `CostTapType.doListPayment()`.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        game.tap(cid);
    }
    // TODO: Fire TapAll trigger — currently done by caller
    true
}

pub fn refund(game: &mut GameState, cards: &[CardId]) {
    for &cid in cards {
        game.untap(cid);
    }
}

pub const HASH_LKI: &str = "Tapped";
pub const HASH_CARDS: &str = "TappedCards";
