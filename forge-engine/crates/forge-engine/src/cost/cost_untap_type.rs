//! Untap other permanents of a type as a cost. Mirrors Java's `CostUntapType`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        game.untap(cid);
    }
    // TODO: Fire UntapAll trigger — currently done by caller
    true
}

pub fn refund(game: &mut GameState, cards: &[CardId]) {
    for &cid in cards {
        game.tap(cid);
    }
}

pub const HASH_LKI: &str = "Untapped";
pub const HASH_CARDS: &str = "UntappedCards";
