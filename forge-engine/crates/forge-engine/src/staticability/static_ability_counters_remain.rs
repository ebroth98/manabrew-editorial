use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn counters_remain(cards: &[Card], card: &Card, destination: ZoneType) -> bool {
    if matches!(
        destination,
        ZoneType::Library | ZoneType::Hand | ZoneType::None
    ) {
        return false;
    }
    for source in cards {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CountersRemain)
        {
            let active = source.zone == ZoneType::Battlefield
                || (source.id == card.id
                    && st_ab
                        .params
                        .get(keys::EFFECT_ZONE)
                        .is_some_and(|z| z.eq_ignore_ascii_case("All")));
            if !active {
                continue;
            }
            if matches_valid_card(st_ab.params.selector(keys::VALID_CARD), card, source) {
                return true;
            }
        }
    }
    false
}

pub fn apply_counters_remain_ability(
    st_ab: &crate::staticability::StaticAbility,
    source: &Card,
    card: &Card,
    destination: ZoneType,
) -> bool {
    if matches!(
        destination,
        ZoneType::Library | ZoneType::Hand | ZoneType::None
    ) {
        return false;
    }
    let active = source.zone == ZoneType::Battlefield
        || (source.id == card.id
            && st_ab
                .params
                .get(keys::EFFECT_ZONE)
                .is_some_and(|z| z.eq_ignore_ascii_case("All")));
    active && matches_valid_card(st_ab.params.selector(keys::VALID_CARD), card, source)
}

fn matches_valid_card(
    valid: Option<&crate::parsing::CompiledSelector>,
    card: &Card,
    source: &Card,
) -> bool {
    valid_filter::matches_valid_card_selector_opt(valid, card, source)
}
