use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

/// Check if a player can spend mana as though it were any color/type
/// when casting a particular spell.
///
/// Mirrors Java's `StaticAbilityManaConvert.manaConvert()`.
///
/// Returns true if any active ManaConvert static on the battlefield allows
/// the player to spend mana freely for the given card.
pub fn can_spend_mana_as_any_color(
    cards: &[CardInstance],
    player: PlayerId,
    spell_card: &CardInstance,
) -> bool {
    for source in cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::ManaConvert)
        {
            // Check ValidPlayer$
            if let Some(valid_player) = st_ab.params.get(keys::VALID_PLAYER) {
                if !matches_valid_player(valid_player, player, source) {
                    continue;
                }
            }

            // Check ValidCard$ (what spell this applies to)
            if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
                if !matches_valid_card(valid_card, spell_card, source) {
                    continue;
                }
            }

            // Check ManaConversion$ — we support the dominant pattern
            if let Some(conversion) = st_ab.params.get(keys::MANA_CONVERSION) {
                if conversion.contains("AnyColor") || conversion.contains("AnyType") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn mana_convert(
    cards: &[CardInstance],
    player: PlayerId,
    spell_card: &CardInstance,
) -> bool {
    can_spend_mana_as_any_color(cards, player, spell_card)
}

pub fn check_mana_convert(
    st_ab: &crate::staticability::StaticAbility,
    source: &CardInstance,
    player: PlayerId,
    spell_card: &CardInstance,
) -> bool {
    if let Some(valid_player) = st_ab.params.get(keys::VALID_PLAYER) {
        if !matches_valid_player(valid_player, player, source) {
            return false;
        }
    }
    if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
        if !matches_valid_card(valid_card, spell_card, source) {
            return false;
        }
    }
    st_ab
        .params
        .get(keys::MANA_CONVERSION)
        .is_some_and(|conversion| conversion.contains("AnyColor") || conversion.contains("AnyType"))
}

fn matches_valid_player(valid: &str, player: PlayerId, source: &CardInstance) -> bool {
    for part in valid.split(',') {
        match part.trim() {
            "You" => {
                if player == source.controller {
                    return true;
                }
            }
            "Player" => return true,
            _ => {}
        }
    }
    false
}

fn matches_valid_card(valid: &str, card: &CardInstance, source: &CardInstance) -> bool {
    for part in valid.split(',') {
        let part = part.trim();
        if part == "Card" || part == "Spell" {
            return true;
        }
        // Handle "Type.Qualifier" patterns
        let segments: Vec<&str> = part.split('.').collect();
        let base = segments[0];
        let base_ok = match base {
            "Creature" => card.is_creature(),
            "Artifact" => card.type_line.is_artifact(),
            "Enchantment" => card.type_line.is_enchantment(),
            "Instant" => card.type_line.is_instant(),
            "Sorcery" => card.type_line.is_sorcery(),
            "Planeswalker" => card.type_line.is_planeswalker(),
            "Land" => card.is_land(),
            _ => card.type_line.has_subtype(base),
        };
        if !base_ok {
            continue;
        }
        // Check qualifiers
        let mut qualifier_ok = true;
        for &seg in &segments[1..] {
            match seg {
                "YouCtrl" | "YouControl" => {
                    if card.controller != source.controller {
                        qualifier_ok = false;
                    }
                }
                _ => {} // ignore unknown qualifiers
            }
        }
        if qualifier_ok {
            return true;
        }
    }
    false
}
