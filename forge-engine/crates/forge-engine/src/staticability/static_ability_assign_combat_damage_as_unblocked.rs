use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn assign_as_unblocked(cards: &[Card], card: &Card, optional: bool) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::AssignCombatDamageAsUnblocked)
        {
            let has_optional = st_ab.params.has(keys::OPTIONAL);
            if has_optional && !optional {
                continue;
            } else if !has_optional && optional {
                continue;
            }
            if matches_valid_card(st_ab.params.selector(keys::VALID_CARD), card, source) {
                return true;
            }
        }
    }
    false
}

pub fn has_optional_assign_as_unblocked(cards: &[Card], card: &Card) -> bool {
    cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .flat_map(|source| {
            source
                .static_abilities
                .iter()
                .filter(move |sa| sa.mode == StaticMode::AssignCombatDamageAsUnblocked)
                .map(move |sa| (source, sa))
        })
        .any(|(source, sa)| {
            sa.params.has(keys::OPTIONAL)
                && matches_valid_card(sa.params.selector(keys::VALID_CARD), card, source)
        })
}

pub fn has_mandatory_assign_as_unblocked(cards: &[Card], card: &Card) -> bool {
    cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .flat_map(|source| {
            source
                .static_abilities
                .iter()
                .filter(move |sa| sa.mode == StaticMode::AssignCombatDamageAsUnblocked)
                .map(move |sa| (source, sa))
        })
        .any(|(source, sa)| {
            !sa.params.has(keys::OPTIONAL)
                && matches_valid_card(sa.params.selector(keys::VALID_CARD), card, source)
        })
}

/// Java parity alias for `assign_as_unblocked`.
pub fn assign_combat_damage_as_unblocked(cards: &[Card], card: &Card) -> bool {
    has_mandatory_assign_as_unblocked(cards, card)
}

fn matches_valid_card(
    valid: Option<&crate::parsing::CompiledSelector>,
    card: &Card,
    source: &Card,
) -> bool {
    valid_filter::matches_valid_card_selector_opt(valid, card, source)
}
