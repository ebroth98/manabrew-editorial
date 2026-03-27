//! Parity shim for Java `CostRemoveCounter`.

pub fn pay_as_decided(
    game: &mut crate::game::GameState,
    source: crate::ids::CardId,
    amount: i32,
    counter_type: &crate::card::CounterType,
) -> bool {
    crate::cost::cost_sub_counter::pay_as_decided(game, source, amount, counter_type)
}

pub fn refund(
    game: &mut crate::game::GameState,
    source: crate::ids::CardId,
    amount: i32,
    counter_type: &crate::card::CounterType,
) {
    crate::cost::cost_sub_counter::refund(game, source, amount, counter_type);
}

pub fn can_pay(
    game: &crate::game::GameState,
    _available_mana: &crate::mana::ManaPool,
    source: crate::ids::CardId,
    _player: crate::ids::PlayerId,
    _ability: Option<&crate::spellability::SpellAbility>,
    part: &super::CostPart,
) -> bool {
    crate::cost::cost_sub_counter::can_pay(game, source, part)
}

pub fn pay_with_decision(
    game: &mut crate::game::GameState,
    player: crate::ids::PlayerId,
    source: crate::ids::CardId,
    part: &super::CostPart,
    _decision: &crate::cost::payment_decision::PaymentDecision,
) -> bool {
    let super::CostPart::SubCounter {
        amount,
        counter_type,
    } = part
    else {
        return false;
    };
    let resolved = super::resolve_dynamic_amount(game, source, player, *amount);
    pay_as_decided(game, source, resolved, counter_type)
}

pub fn payment_order(part: &super::CostPart) -> i32 {
    part.payment_order()
}
