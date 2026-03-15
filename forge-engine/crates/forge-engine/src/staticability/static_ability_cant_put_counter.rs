use forge_foundation::ZoneType;

use crate::card::{CardInstance, CounterType};
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn any_cant_put_counter_on_card(
    cards: &[CardInstance],
    target: &CardInstance,
    counter_type: &CounterType,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPutCounter)
        {
            if !counter_type_matches(
                st_ab.params.get("CounterType").map(String::as_str),
                &counter_type,
            ) {
                continue;
            }
            if !matches_valid_card(
                st_ab.params.get("ValidCard").map(String::as_str),
                target,
                source,
            ) {
                continue;
            }
            if st_ab.params.contains_key("ValidPlayer") {
                continue;
            }
            return true;
        }
    }
    false
}

pub fn any_cant_put_counter_on_player(
    cards: &[CardInstance],
    player: PlayerId,
    counter_type: &CounterType,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPutCounter)
        {
            if !counter_type_matches(
                st_ab.params.get("CounterType").map(String::as_str),
                &counter_type,
            ) {
                continue;
            }
            if !matches_valid_player(
                st_ab.params.get("ValidPlayer").map(String::as_str),
                player,
                source.controller,
            ) {
                continue;
            }
            if st_ab.params.contains_key("ValidCard") {
                continue;
            }
            return true;
        }
    }
    false
}

fn counter_type_matches(param: Option<&str>, ct: &CounterType) -> bool {
    match param {
        None => true,
        Some(s) => parse_counter_type_opt(s).map(|p| p == *ct).unwrap_or(true),
    }
}

fn matches_valid_player(
    valid: Option<&str>,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Player") => true,
        Some(v) if v.eq_ignore_ascii_case("You") || v.eq_ignore_ascii_case("YouCtrl") => {
            player == source_controller
        }
        Some(v) if v.eq_ignore_ascii_case("Opponent") || v.eq_ignore_ascii_case("OppCtrl") => {
            player != source_controller
        }
        _ => true,
    }
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
    let upper = s.to_uppercase();
    match upper.as_str() {
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
        _ => Some(CounterType::Named(upper)),
    }
}
