//! Remove any counter type from permanents as a cost. Mirrors Java's `CostRemoveAnyCounter`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::CardId;

/// Pay by removing counters from selected permanents.
/// The caller provides the (card, counter_type, amount) decisions.
/// Mirrors Java's `CostRemoveAnyCounter.payAsDecided()` which iterates
/// `decision.counterTable`.
pub fn pay_as_decided(game: &mut GameState, removals: &[(CardId, CounterType, i32)]) -> bool {
    for &(cid, ref ct, amt) in removals {
        game.card_mut(cid).remove_counter(ct, amt);
    }
    true
}
