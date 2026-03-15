//! Exile from controlled battlefield or graveyard as a combined cost.
//! Used by Craft abilities. No direct Java CostExileCtrlOrGrave class — this is
//! a Rust-side variant that combines CostExile battlefield + graveyard sources.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

/// Execute exile payment for selected cards.
/// Cards may come from battlefield or graveyard.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        game.move_card(cid, ZoneType::Exile, owner);
    }
    true
}

pub const HASH_LKI: &str = "ExiledCtrlOrGrave";
pub const HASH_CARDS: &str = "ExiledCtrlOrGraveCards";
