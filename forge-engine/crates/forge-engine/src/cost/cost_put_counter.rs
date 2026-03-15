//! Put counters on permanents as a cost. Mirrors Java's `CostPutCounter`.
//!
//! Java's `CostPutCounter` extends `CostPartWithList` and manages counter
//! placement on source or target permanents. It also handles ETB replacement
//! effects where counters are placed as the card enters the battlefield.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::CardId;

/// Add counters to the source.
/// Mirrors Java's `CostPutCounter.doPayment()`.
pub fn pay_as_decided(
    game: &mut GameState,
    source: CardId,
    amount: i32,
    counter_type: &CounterType,
) -> bool {
    game.card_mut(source).add_counter(counter_type, amount);
    // TODO: Fire counter placement triggers via GameEntityCounterTable
    // Java's CostPutCounter.triggerCounterPutAll() handles this
    true
}

/// Refund by removing the placed counters.
/// Mirrors Java's `CostPutCounter.refund()`.
pub fn refund(
    game: &mut GameState,
    source: CardId,
    amount: i32,
    counter_type: &CounterType,
) {
    game.card_mut(source).remove_counter(counter_type, amount);
}

pub const HASH_LKI: &str = "CounterPut";
pub const HASH_CARDS: &str = "CounterPutCards";
