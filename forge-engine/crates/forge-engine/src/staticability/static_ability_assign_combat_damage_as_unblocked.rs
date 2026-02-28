use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::staticability::StaticMode;

pub fn assign_as_unblocked(cards: &[CardInstance], card: &CardInstance, optional: bool) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::AssignCombatDamageAsUnblocked)
        {
            let has_optional = st_ab.params.contains_key("Optional");
            if has_optional && !optional {
                continue;
            } else if !has_optional && optional {
                continue;
            }
            if matches_valid_card(st_ab.params.get("ValidCard").map(String::as_str), card, source) {
                return true;
            }
        }
    }
    false
}

pub fn has_optional_assign_as_unblocked(cards: &[CardInstance], card: &CardInstance) -> bool {
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
            sa.params.contains_key("Optional")
                && matches_valid_card(sa.params.get("ValidCard").map(String::as_str), card, source)
        })
}

pub fn has_mandatory_assign_as_unblocked(cards: &[CardInstance], card: &CardInstance) -> bool {
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
            !sa.params.contains_key("Optional")
                && matches_valid_card(sa.params.get("ValidCard").map(String::as_str), card, source)
        })
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
