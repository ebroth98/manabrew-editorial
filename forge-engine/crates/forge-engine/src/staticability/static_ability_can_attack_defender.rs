use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn can_attack_defender(
    cards: &[CardInstance],
    card: &CardInstance,
    defender: PlayerId,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CanAttackDefender)
        {
            if !matches_valid_card(st_ab.params.get("ValidCard").map(String::as_str), card, source) {
                continue;
            }
            if !matches_valid_attacked(
                st_ab.params.get("ValidAttacked").map(String::as_str),
                defender,
                source.controller,
            ) {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Card.IsRemembered") => {
            source.remembered_cards.contains(&card.id)
        }
        _ => true,
    }
}

fn matches_valid_attacked(
    valid: Option<&str>,
    defender: PlayerId,
    source_controller: PlayerId,
) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Player") => true,
        Some(v) if v.eq_ignore_ascii_case("You") || v.eq_ignore_ascii_case("YouCtrl") => {
            defender == source_controller
        }
        Some(v) if v.eq_ignore_ascii_case("Opponent") || v.eq_ignore_ascii_case("OppCtrl") => {
            defender != source_controller
        }
        _ => true,
    }
}

