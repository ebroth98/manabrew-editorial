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

pub fn sort_replacement_descriptions_with_indices(
    descriptions: &[String],
) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = descriptions.iter().cloned().enumerate().collect();
    out.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    out
}
