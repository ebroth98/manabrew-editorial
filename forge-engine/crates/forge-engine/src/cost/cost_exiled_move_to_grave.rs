//! Move exiled cards to graveyard as a cost. Mirrors Java's `CostExiledMoveToGrave`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

/// Execute the exiled-move-to-grave payment for selected cards.
/// Mirrors Java's `CostExiledMoveToGrave.doPayment()`.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        game.move_card(cid, ZoneType::Graveyard, owner);
    }
    true
}

pub const HASH_LKI: &str = "MovedToGrave";
pub const HASH_CARDS: &str = "MovedToGraveCards";
