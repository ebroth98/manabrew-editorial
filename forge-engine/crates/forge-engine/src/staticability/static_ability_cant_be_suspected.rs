use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn cant_be_suspected(cards: &[Card], card: &Card) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantBeSuspected && sa.zones_check(source.zone))
        {
            if valid_filter::matches_valid_card_selector_opt(
                st_ab.params.selector(keys::VALID_CARD),
                card,
                source,
            ) {
                return true;
            }
        }
    }
    false
}
