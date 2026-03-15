//! Promise a gift to an opponent as a cost. Mirrors Java's `CostPromiseGift`.
//!
//! The player chooses an opponent to receive the gift. The chosen opponent
//! is stored on the host card via `set_promised_gift()`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Execute the promise gift payment.
/// Mirrors Java's `CostPromiseGift.payAsDecided()`.
/// Sets the promised gift recipient on the host card.
pub fn pay_as_decided(game: &mut GameState, source: CardId, recipient: Option<PlayerId>) -> bool {
    match recipient {
        Some(pid) => {
            game.card_mut(source).promised_gift = Some(pid);
            true
        }
        None => {
            game.card_mut(source).promised_gift = None;
            false
        }
    }
}

/// Get potential gift recipients (opponents of payer).
pub fn get_potential_players(game: &GameState, payer: PlayerId) -> Vec<PlayerId> {
    game.alive_players()
        .into_iter()
        .filter(|&pid| pid != payer)
        .collect()
}
