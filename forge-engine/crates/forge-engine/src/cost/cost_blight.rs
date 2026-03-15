//! Blight as a cost — put -1/-1 counters on creatures you control.
//! Mirrors Java's `CostBlight` which extends `CostPutCounter`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::CardId;

/// Execute blight payment for selected creatures.
/// Puts a -1/-1 counter on each chosen creature.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        game.card_mut(cid).add_counter(&CounterType::M1M1, 1);
    }
    true
}

/// Refund blight payment — remove the -1/-1 counters.
pub fn refund(game: &mut GameState, cards: &[CardId]) {
    for &cid in cards {
        game.card_mut(cid).remove_counter(&CounterType::M1M1, 1);
    }
}
