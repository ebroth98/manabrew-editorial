//! Behold (reveal from hand or battlefield) as a cost. Mirrors Java's `CostBehold`.
//!
//! Behold extends CostReveal in Java, allowing reveal from Hand or Battlefield.
//! Optionally exiles the revealed cards.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

/// Execute behold payment for selected cards.
/// Cards have already been chosen by the agent.
/// If `exile` is true, moves revealed cards to exile.
pub fn pay_as_decided_cards(
    game: &mut GameState,
    cards: &[CardId],
    exile: bool,
) -> bool {
    if exile {
        for &cid in cards {
            let owner = game.card(cid).owner;
            game.move_card(cid, ZoneType::Exile, owner);
        }
    }
    // Non-exile behold just reveals — no zone change needed
    true
}

pub const HASH_LKI: &str = "Beheld";
pub const HASH_CARDS: &str = "BeheldCards";
