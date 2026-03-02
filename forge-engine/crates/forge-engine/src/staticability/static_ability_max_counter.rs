use forge_foundation::ZoneType;

use crate::card::{CardInstance, CounterType};
use crate::staticability::StaticMode;

pub fn max_counter(cards: &[CardInstance], target: &CardInstance, counter_type: CounterType) -> Option<i32> {
    let mut result: Option<i32> = None;
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::MaxCounter)
        {
            if let Some(s) = st_ab.params.get("CounterType") {
                if let Some(parsed) = parse_counter_type_opt(s) {
                    if parsed != counter_type {
                        continue;
                    }
                }
            }
            if !matches_valid_card(st_ab.params.get("ValidCard").map(String::as_str), target, source) {
                continue;
            }
            let value = st_ab
                .params
                .get("MaxNum")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            result = Some(result.map_or(value, |v| v.min(value)));
        }
    }
    result
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") || v.eq_ignore_ascii_case("Permanent") => true,
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        _ => true,
    }
}

fn parse_counter_type_opt(s: &str) -> Option<CounterType> {
    match s.to_uppercase().as_str() {
        "POISON" => Some(CounterType::Poison),
        "P1P1" | "+1/+1" => Some(CounterType::P1P1),
        "M1M1" | "-1/-1" => Some(CounterType::M1M1),
        "LOYALTY" => Some(CounterType::Loyalty),
        "CHARGE" => Some(CounterType::Charge),
        "QUEST" => Some(CounterType::Quest),
        "STUDY" => Some(CounterType::Study),
        "AGE" => Some(CounterType::Age),
        "FADE" => Some(CounterType::Fade),
        "TIME" => Some(CounterType::Time),
        "DEPLETION" => Some(CounterType::Depletion),
        "STORAGE" => Some(CounterType::Storage),
        "MINING" => Some(CounterType::Mining),
        "BRICK" => Some(CounterType::Brick),
        "LEVEL" => Some(CounterType::Level),
        "LORE" => Some(CounterType::Lore),
        "PAGE" => Some(CounterType::Page),
        "DREAM" => Some(CounterType::Dream),
        _ => None,
    }
}
