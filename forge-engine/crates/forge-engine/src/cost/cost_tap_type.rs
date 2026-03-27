//! Tap other permanents of a type as a cost. Mirrors Java's `CostTapType`.

use crate::game::GameState;
use crate::ids::CardId;

/// Pay by tapping the selected cards.
/// Cards are passed in (already selected by agent).
/// Mirrors Java's `CostTapType.doListPayment()`.
pub fn pay_as_decided_cards(game: &mut GameState, cards: &[CardId]) -> bool {
    for &cid in cards {
        game.tap(cid);
    }
    // TODO: Fire TapAll trigger — currently done by caller
    true
}

pub fn refund(game: &mut GameState, cards: &[CardId]) {
    for &cid in cards {
        game.untap(cid);
    }
}

pub const HASH_LKI: &str = "Tapped";
pub const HASH_CARDS: &str = "TappedCards";

pub fn can_pay(
    game: &crate::game::GameState,
    _available_mana: &crate::mana::ManaPool,
    source: crate::ids::CardId,
    player: crate::ids::PlayerId,
    _ability: Option<&crate::spellability::SpellAbility>,
    part: &super::CostPart,
) -> bool {
    let super::CostPart::TapType {
        amount,
        type_filter,
        min_total_power,
    } = part
    else {
        return false;
    };
    let targets = super::get_tap_type_targets(game, player, type_filter, source);
    if let Some(power_threshold) = min_total_power {
        let total_power: i32 = targets.iter().map(|&cid| game.card(cid).power()).sum();
        total_power >= *power_threshold
    } else {
        (targets.len() as i32) >= *amount
    }
}

pub fn pay_with_decision(
    game: &mut GameState,
    _player: crate::ids::PlayerId,
    _source: CardId,
    _part: &super::CostPart,
    decision: &crate::cost::payment_decision::PaymentDecision,
) -> bool {
    if let crate::cost::payment_decision::PaymentDecision::Cards(cards) = decision {
        return pay_as_decided_cards(game, cards);
    }
    false
}
