use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::staticability::StaticMode;

pub fn damage_not_removed(cards: &[CardInstance], card: &CardInstance) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::NoCleanupDamage)
        {
            if matches_valid_card(
                st_ab.params.get("ValidCard").map(String::as_str),
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
    let filter = match valid {
        None => return true,
        Some(v) => v,
    };

    // Split on '.' for compound filters (e.g. "Creature.OppCtrl")
    let parts: Vec<&str> = filter.split('.').collect();
    let type_part = parts[0];

    let type_matches = match type_part {
        "Card" | "Permanent" => true,
        "Creature" => card.is_creature(),
        _ => true,
    };
    if !type_matches {
        return false;
    }

    // Check qualifiers
    for &qualifier in &parts[1..] {
        for sub in qualifier.split('+') {
            match sub {
                "Self" => {
                    if card.id != source.id {
                        return false;
                    }
                }
                "OppCtrl" => {
                    if card.controller == source.controller {
                        return false;
                    }
                }
                "YouCtrl" => {
                    if card.controller != source.controller {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    true
}
