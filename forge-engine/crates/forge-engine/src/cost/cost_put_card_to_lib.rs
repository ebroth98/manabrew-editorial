//! Put cards to library as a cost. Mirrors Java's `CostPutCardToLib`.
//!
//! Covers PutCardToLibFromHand, PutCardToLibFromGrave, PutCardToLibFromSameGrave.
//! Java uses `from` zone and `sameZone` flag to distinguish variants.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

/// Execute put-to-library for self (CARDNAME/NICKNAME).
/// Mirrors Java's `CostPutCardToLib.doPayment()` for self.
pub fn pay_as_decided_self(game: &mut GameState, source: CardId, lib_pos: i32) -> bool {
    let owner = game.card(source).owner;
    if lib_pos == 0 {
        game.move_card(source, ZoneType::Library, owner);
    } else {
        game.put_on_bottom_of_library(source, owner);
    }
    true
}

/// Execute put-to-library for selected cards.
/// Mirrors Java's `CostPutCardToLib.doPayment()`.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId], lib_pos: i32) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        if lib_pos == 0 {
            game.move_card(cid, ZoneType::Library, owner);
        } else {
            game.put_on_bottom_of_library(cid, owner);
        }
    }
    true
}

pub const HASH_LKI: &str = "CardPutToLib";
pub const HASH_CARDS: &str = "CardPutToLibCards";
