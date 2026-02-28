use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::staticability::StaticMode;

pub fn colorless_damage_source(cards: &[CardInstance], source_card: &CardInstance) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::ColorlessDamageSource)
        {
            if matches_valid_card(
                st_ab.params.get("ValidCard").map(String::as_str),
                source_card,
                source,
            ) {
                return true;
            }
        }
    }
    false
}

pub fn source_has_color(
    cards: &[CardInstance],
    source_card: &CardInstance,
    color_name: &str,
) -> bool {
    if colorless_damage_source(cards, source_card) {
        return color_name.eq_ignore_ascii_case("colorless");
    }
    match color_name.to_ascii_lowercase().as_str() {
        "white" => source_card.color.has_white(),
        "blue" => source_card.color.has_blue(),
        "black" => source_card.color.has_black(),
        "red" => source_card.color.has_red(),
        "green" => source_card.color.has_green(),
        "colorless" => source_card.color.is_colorless(),
        _ => false,
    }
}

pub fn target_is_protected_from_source(
    cards: &[CardInstance],
    target: &CardInstance,
    source: &CardInstance,
) -> bool {
    for prot in target.get_protections() {
        match prot.as_str() {
            "white" | "blue" | "black" | "red" | "green" | "colorless" => {
                if source_has_color(cards, source, &prot) {
                    return true;
                }
            }
            "artifacts" => {
                if source.type_line.is_artifact() {
                    return true;
                }
            }
            "creatures" => {
                if source.type_line.is_creature() {
                    return true;
                }
            }
            "enchantments" => {
                if source.type_line.is_enchantment() {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    let Some(expr) = valid else {
        return true;
    };
    expr.split(',').any(|clause| {
        clause
            .split('+')
            .flat_map(|s| s.split('.'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .all(|tok| match tok {
                "Card" => true,
                "Permanent" => card.zone == ZoneType::Battlefield,
                "Spell" => card.zone == ZoneType::Stack,
                "Creature" => card.is_creature(),
                "Card.Self" => card.id == source.id,
                "Black" => card.color.has_black(),
                "Red" => card.color.has_red(),
                "Blue" => card.color.has_blue(),
                "White" => card.color.has_white(),
                "Green" => card.color.has_green(),
                "inZoneBattlefield" => card.zone == ZoneType::Battlefield,
                "inZoneStack" => card.zone == ZoneType::Stack,
                _ => true,
            })
    })
}
