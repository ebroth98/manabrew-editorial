//! Give control of permanents as a cost. Mirrors Java's `CostGainControl`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Pay by transferring control of selected permanents.
/// Mirrors Java's `CostGainControl.doPayment()` → `card.addTempController(payer)`.
pub fn pay_as_decided_cards(
    game: &mut GameState,
    cards: &[CardId],
    new_controller: PlayerId,
) -> bool {
    for &cid in cards {
        game.card_mut(cid).controller = new_controller;
    }
    true
}

pub const HASH_LKI: &str = "ControlGained";
pub const HASH_CARDS: &str = "ControlGainedCards";
