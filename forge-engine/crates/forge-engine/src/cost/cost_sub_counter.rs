//! Remove counters from source as a cost. Mirrors Java's `CostRemoveCounter`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::CardId;

/// Remove counters from the source.
/// Mirrors Java's `CostRemoveCounter.payAsDecided()`.
pub fn pay_as_decided(
    game: &mut GameState,
    source: CardId,
    amount: i32,
    counter_type: &CounterType,
) -> bool {
    game.card_mut(source).remove_counter(counter_type, amount);
    true
}

/// Refund by adding counters back.
pub fn refund(
    game: &mut GameState,
    source: CardId,
    amount: i32,
    counter_type: &CounterType,
) {
    game.card_mut(source).add_counter(counter_type, amount);
}
