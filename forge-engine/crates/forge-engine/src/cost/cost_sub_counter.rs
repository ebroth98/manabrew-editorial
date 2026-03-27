//! Remove counters from source as a cost. Mirrors Java's `CostRemoveCounter`.

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
pub fn refund(game: &mut GameState, source: CardId, amount: i32, counter_type: &CounterType) {
    game.card_mut(source).add_counter(counter_type, amount);
}

pub fn can_pay(game: &GameState, source: CardId, part: &super::CostPart) -> bool {
    let super::CostPart::SubCounter {
        amount,
        counter_type,
    } = part
    else {
        return false;
    };
    let card = game.card(source);
    if card.zone != forge_foundation::ZoneType::Battlefield || card.phased_out {
        return false;
    }
    card.counter_count(counter_type) >= *amount
}
