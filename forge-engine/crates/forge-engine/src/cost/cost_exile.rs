//! Exile cards as a cost. Mirrors Java's `CostExile`.
//!
//! Covers ExileFromHand, ExileFromGrave, ExileFromTop, ExileSameGrave,
//! and ExileFromBattlefield variants. Java uses `zoneMode` to distinguish;
//! Rust uses separate CostPart variants for some (ExileFromAnyGrave, ExileFromSameGrave).
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::CardId;

/// Execute exile of self (CARDNAME/OriginalHost).
/// Mirrors Java's `CostExile` doPayment for self-exile.
pub fn pay_as_decided_self(game: &mut GameState, source: CardId) -> bool {
    let owner = game.card(source).owner;
    game.move_card(source, ZoneType::Exile, owner);
    true
}

/// Execute typed exile (non-self).
/// Cards to exile are passed in (already selected by agent).
/// Mirrors Java's `CostExile.doListPayment()`.
pub fn pay_as_decided_cards(
    game: &mut GameState,
    cards: &[CardId],
) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        game.move_card(cid, ZoneType::Exile, owner);
    }
    true
}

/// Hash keys for LKI/card tracking lists.
pub const HASH_LKI: &str = "Exiled";
pub const HASH_CARDS: &str = "ExiledCards";
