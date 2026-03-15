//! Reveal a previously chosen player or type as a cost. Mirrors Java's `CostRevealChosen`.
//!
//! This cost requires the source card to have a chosen player or type set,
//! and the controller must be the activator.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Execute the reveal-chosen payment.
/// Mirrors Java's `CostRevealChosen.payAsDecided()`.
/// Reveals the chosen player or type on the host card.
pub fn pay_as_decided(game: &mut GameState, source: CardId, reveal_type: &str) -> bool {
    let card = game.card_mut(source);
    if reveal_type == "Player" {
        if card.chosen_player.is_some() {
            // Mark as revealed (Java calls host.revealChosenPlayer())
            return true;
        }
    } else if reveal_type == "Type" {
        if card.chosen_type.is_some() {
            // Mark as revealed (Java calls host.revealChosenType())
            return true;
        }
    }
    false
}
