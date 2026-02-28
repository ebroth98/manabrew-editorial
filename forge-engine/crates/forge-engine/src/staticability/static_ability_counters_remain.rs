use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::staticability::StaticMode;

pub fn counters_remain(cards: &[CardInstance], card: &CardInstance, destination: ZoneType) -> bool {
    if matches!(destination, ZoneType::Library | ZoneType::Hand | ZoneType::None) {
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
                        .get("EffectZone")
                        .is_some_and(|z| z.eq_ignore_ascii_case("All")));
            if !active {
                continue;
            }
            if matches_valid_card(st_ab.params.get("ValidCard").map(String::as_str), card, source) {
                return true;
            }
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") || v.eq_ignore_ascii_case("Permanent") => true,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        _ => true,
    }
}
