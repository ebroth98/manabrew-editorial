use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn is_wither_damage(cards: &[Card], source_card: &Card) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::WitherDamage)
        {
            if matches_valid_card(st_ab.params.selector(keys::VALID_CARD), source_card, source) {
                return true;
            }
        }
    }
    false
}

fn matches_valid_card(
    valid: Option<&crate::parsing::CompiledSelector>,
    card: &Card,
    source: &Card,
) -> bool {
    valid_filter::matches_valid_card_selector_opt(valid, card, source)
}
