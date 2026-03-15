use forge_engine_core::ids::CardId;

pub fn sort_cards_by_name_then_id(
    cards: &[CardId],
    mut card_name: impl FnMut(CardId) -> String,
    mut parity_id: impl FnMut(CardId) -> u32,
) -> Vec<CardId> {
    let mut out: Vec<CardId> = cards.to_vec();
    out.sort_by(|a, b| {
        let an = card_name(*a);
        let bn = card_name(*b);
        an.cmp(&bn).then_with(|| parity_id(*a).cmp(&parity_id(*b)))
    });
    out
}
