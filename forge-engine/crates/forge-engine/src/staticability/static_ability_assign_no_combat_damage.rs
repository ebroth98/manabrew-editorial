use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn assign_no_combat_damage(cards: &[CardInstance], card: &CardInstance) -> bool {
    for source in cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::AssignNoCombatDamage)
        {
            if matches_valid_card(
                st_ab.params.get(keys::VALID_CARD),
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
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") || v.eq_ignore_ascii_case("Permanent") => true,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        Some(v) if v.eq_ignore_ascii_case("Card.IsRemembered") => {
            source.remembered_cards.contains(&card.id)
        }
        Some(v) if v.eq_ignore_ascii_case("Card.EffectSource") => {
            source.effect_source == Some(card.id)
        }
        Some(v) => crate::card::valid_filter::matches_valid_card(v, card, source),
    }
}
