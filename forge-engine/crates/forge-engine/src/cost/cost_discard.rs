//! Discard cards as a cost. Mirrors Java's `CostDiscard`.
//!
//! Java's `CostDiscard` extends `CostPartWithList` and uses `doPayment()`
//! to call `payer.discard(targetCard, ...)`. It also fires `DiscardedAll`
//! trigger after all discards.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Execute discard of self (CARDNAME).
/// Mirrors Java's `CostDiscard.doPayment()` for self-discard.
pub fn pay_as_decided_self(game: &mut GameState, source: CardId, player: PlayerId) -> bool {
    let owner = game.card(source).owner;
    game.move_card(source, ZoneType::Graveyard, owner);
    let _ = player;
    // TODO: Fire Discarded trigger — currently done by caller
    true
}

/// Execute typed discard (non-self).
/// Cards to discard are passed in (already selected by agent).
/// Mirrors Java's `CostDiscard.doPayment()`.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId], _player: PlayerId) -> bool {
    for &cid in cards {
        let owner = game.card(cid).owner;
        game.move_card(cid, ZoneType::Graveyard, owner);
        // TODO: Fire Discarded trigger per card
    }
    // TODO: Fire DiscardedAll trigger after all discards
    true
}

/// Hash keys for LKI/card tracking lists.
pub const HASH_LKI: &str = "Discarded";
pub const HASH_CARDS: &str = "DiscardedCards";
