use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card, CounterType};
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn any_cant_put_counter_on_card(
    cards: &[Card],
    target: &Card,
    counter_type: &CounterType,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPutCounter)
        {
            if !counter_type_matches(st_ab.params.get(keys::COUNTER_TYPE), &counter_type) {
                continue;
            }
            if !matches_valid_card(st_ab.params.selector(keys::VALID_CARD), target, source) {
                continue;
            }
            if st_ab.params.has(keys::VALID_PLAYER) {
                continue;
            }
            return true;
        }
    }
    false
}

pub fn any_cant_put_counter_on_player(
    cards: &[Card],
    player: PlayerId,
    counter_type: &CounterType,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPutCounter)
        {
            if !counter_type_matches(st_ab.params.get(keys::COUNTER_TYPE), &counter_type) {
                continue;
            }
            if !matches_valid_player(
                st_ab.params.selector(keys::VALID_PLAYER),
                player,
                source.controller,
            ) {
                continue;
            }
            if st_ab.params.has(keys::VALID_CARD) {
                continue;
            }
            return true;
        }
    }
    false
}

pub fn any_cant_put_counter(
    cards: &[Card],
    target_card: Option<&Card>,
    target_player: Option<PlayerId>,
    counter_type: &CounterType,
) -> bool {
    if let Some(card) = target_card {
        return any_cant_put_counter_on_card(cards, card, counter_type);
    }
    if let Some(player) = target_player {
        return any_cant_put_counter_on_player(cards, player, counter_type);
    }
    false
}

pub fn apply_cant_put_counter(
    st_ab: &crate::staticability::StaticAbility,
    source: &Card,
    target_card: Option<&Card>,
    target_player: Option<PlayerId>,
    counter_type: &CounterType,
) -> bool {
    if !counter_type_matches(st_ab.params.get(keys::COUNTER_TYPE), counter_type) {
        return false;
    }
    if let Some(card) = target_card {
        if st_ab.params.has(keys::VALID_PLAYER) {
            return false;
        }
        return matches_valid_card(st_ab.params.selector(keys::VALID_CARD), card, source);
    }
    if let Some(player) = target_player {
        if st_ab.params.has(keys::VALID_CARD) {
            return false;
        }
        return matches_valid_player(
            st_ab.params.selector(keys::VALID_PLAYER),
            player,
            source.controller,
        );
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
    valid: Option<&crate::parsing::CompiledSelector>,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    valid_filter::matches_valid_player_selector_opt(valid, player, source_controller)
}

fn matches_valid_card(
    valid: Option<&crate::parsing::CompiledSelector>,
    card: &Card,
    source: &Card,
) -> bool {
    valid_filter::matches_valid_card_selector_opt(valid, card, source)
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
