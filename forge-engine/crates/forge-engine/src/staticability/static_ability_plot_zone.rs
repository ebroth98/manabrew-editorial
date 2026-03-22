use forge_foundation::ZoneType;
use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn plot_zone(cards: &[Card], card: &Card) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source.static_abilities.iter().filter(|sa| sa.mode == StaticMode::PlotZone && sa.zones_check(source.zone)) {
            if valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_CARD), card, source) {
                return true;
            }
        }
    }
    false
}
