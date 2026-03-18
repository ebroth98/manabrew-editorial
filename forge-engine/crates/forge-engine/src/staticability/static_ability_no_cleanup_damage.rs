use forge_foundation::ZoneType;

use crate::card::{valid_filter, CardInstance};
use crate::staticability::StaticMode;

pub fn damage_not_removed(cards: &[CardInstance], card: &CardInstance) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::NoCleanupDamage)
        {
            if matches_valid_card(
                st_ab.params.get("ValidCard").map(String::as_str),
                card,
                source,
            ) {
                return true;
            }
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    valid_filter::matches_valid_card_opt(valid, card, source)
}
