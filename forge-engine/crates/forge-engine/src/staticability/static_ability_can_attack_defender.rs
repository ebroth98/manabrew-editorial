use forge_foundation::ZoneType;

use crate::ability::effects::parse_counter_type;
use crate::card::CardInstance;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn can_attack_defender(
    cards: &[CardInstance],
    card: &CardInstance,
    defender: PlayerId,
) -> bool {
    for source in cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CanAttackDefender)
        {
            if !matches_valid_card(
                st_ab.params.get(keys::VALID_CARD),
                card,
                source,
            ) {
                continue;
            }
            if !matches_valid_attacked(
                st_ab.params.get(keys::VALID_ATTACKED),
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
    let filter = match valid {
        None => return true,
        Some(v) => v,
    };

    // Split on '.' for compound filters (e.g. "Card.Self+counters_GE3_P1P1")
    let parts: Vec<&str> = filter.split('.').collect();
    let type_part = parts[0];

    // Check the type portion
    let type_matches = match type_part {
        "Card" => true,
        "Creature" => card.is_creature(),
        _ => true,
    };
    if !type_matches {
        return false;
    }

    // Check qualifiers after the dot, split on '+' for sub-conditions
    for &qualifier in &parts[1..] {
        for sub in qualifier.split('+') {
            if sub.eq_ignore_ascii_case("Self") {
                if card.id != source.id {
                    return false;
                }
            } else if sub.eq_ignore_ascii_case("IsRemembered") {
                if !source.remembered_cards.contains(&card.id) {
                    return false;
                }
            } else if sub.starts_with("counters_") {
                // Parse patterns like "counters_GE3_P1P1"
                if !check_counter_condition(sub, card) {
                    return false;
                }
            }
            // Unknown qualifiers are ignored (permissive)
        }
    }

    true
}

/// Check a counter condition like "counters_GE3_P1P1".
/// Format: counters_{op}{num}_{counter_type}
fn check_counter_condition(condition: &str, card: &CardInstance) -> bool {
    // Strip "counters_" prefix
    let rest = &condition["counters_".len()..];
    // Extract operator (2 chars: GE, GT, LE, LT, EQ, NE)
    if rest.len() < 3 {
        return true;
    }
    let op = &rest[..2];
    // Find the underscore separating the number from the counter type
    let after_op = &rest[2..];
    let (num_str, counter_type_str) = match after_op.find('_') {
        Some(idx) => (&after_op[..idx], &after_op[idx + 1..]),
        None => return true, // malformed, permissive
    };
    let threshold: i32 = num_str.parse().unwrap_or(0);
    let counter_type = parse_counter_type(counter_type_str);
    let count = card.counter_count(&counter_type);
    match op {
        "GE" => count >= threshold,
        "GT" => count > threshold,
        "LE" => count <= threshold,
        "LT" => count < threshold,
        "EQ" => count == threshold,
        "NE" => count != threshold,
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
