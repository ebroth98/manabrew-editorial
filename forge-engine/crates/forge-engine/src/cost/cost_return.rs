//! Return permanents to hand as a cost. Mirrors Java's `CostReturn`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

pub fn pay_as_decided_self(game: &mut GameState, source: CardId) -> bool {
    let owner = game.card(source).owner;
    game.move_card(source, ZoneType::Hand, owner);
    true
}

pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        game.move_card(cid, ZoneType::Hand, owner);
    }
    true
}

pub const HASH_LKI: &str = "Returned";
pub const HASH_CARDS: &str = "ReturnedCards";
