use forge_foundation::ZoneType;

use crate::card::Card;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::parsing::Params;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_target(
    cards: &[Card],
    target: &Card,
    activator: PlayerId,
    source_card: Option<&Card>,
    source_sa: Option<&SpellAbility>,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantTarget)
        {
            if let Some(affected_zone) = st_ab.params.get(keys::AFFECTED_ZONE) {
                let zones: Vec<&str> = affected_zone.split(',').map(|s| s.trim()).collect();
                if !zones.iter().any(|z| zone_matches(target.zone, z)) {
                    continue;
                }
            } else if target.zone != ZoneType::Battlefield {
                continue;
            }

            if !matches_valid_target(st_ab.params.get(keys::VALID_TARGET), target, source) {
                continue;
            }
            if !matches_valid_activator(
                st_ab.params.get(keys::ACTIVATOR),
                activator,
                source.controller,
            ) {
                continue;
            }
            if let Some(valid_sa) = st_ab.params.get(keys::VALID_SA) {
                let Some(sa) = source_sa else {
                    continue;
                };
                if !spell_ability_matches(valid_sa, &sa.ability_text) {
                    continue;
                }
            }
            if let (Some(valid_source), Some(src)) =
                (st_ab.params.get(keys::VALID_SOURCE), source_card)
            {
                if !matches_valid_target(Some(valid_source), src, source) {
                    continue;
                }
            }
            return true;
        }
    }
    false
}

pub fn apply_cant_target_ability(
    st_ab: &crate::staticability::StaticAbility,
    target: &Card,
    source: &Card,
    activator: PlayerId,
    source_card: Option<&Card>,
    source_sa: Option<&SpellAbility>,
) -> bool {
    if let Some(affected_zone) = st_ab.params.get(keys::AFFECTED_ZONE) {
        let zones: Vec<&str> = affected_zone.split(',').map(|s| s.trim()).collect();
        if !zones.iter().any(|z| zone_matches(target.zone, z)) {
            return false;
        }
    } else if target.zone != ZoneType::Battlefield {
        return false;
    }

    if !matches_valid_target(st_ab.params.get(keys::VALID_TARGET), target, source) {
        return false;
    }
    if !matches_valid_activator(
        st_ab.params.get(keys::ACTIVATOR),
        activator,
        source.controller,
    ) {
        return false;
    }
    if let Some(valid_sa) = st_ab.params.get(keys::VALID_SA) {
        let Some(sa) = source_sa else {
            return false;
        };
        if !spell_ability_matches(valid_sa, &sa.ability_text) {
            return false;
        }
    }
    if let (Some(valid_source), Some(src)) = (st_ab.params.get(keys::VALID_SOURCE), source_card) {
        if !matches_valid_target(Some(valid_source), src, source) {
            return false;
        }
    }
    true
}

fn spell_ability_matches(valid_sa: &str, ability_line: &str) -> bool {
    let params = Params::from_raw(ability_line);
    let tokens: Vec<&str> = valid_sa
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return true;
    }
    tokens
        .iter()
        .all(|tok| match tok.to_ascii_lowercase().as_str() {
            "spell" => params.has(keys::SP),
            "istargeting" => params.has(keys::VALID_TGTS),
            "xcost" => {
                params
                    .get(keys::COST)
                    .map(|c| c.contains('X'))
                    .unwrap_or(false)
                    || ability_line.contains("X")
            }
            _ => false,
        })
}

fn zone_matches(zone: ZoneType, zone_str: &str) -> bool {
    match zone_str.to_ascii_lowercase().as_str() {
        "battlefield" => zone == ZoneType::Battlefield,
        "hand" => zone == ZoneType::Hand,
        "graveyard" => zone == ZoneType::Graveyard,
        "library" => zone == ZoneType::Library,
        "exile" => zone == ZoneType::Exile,
        "stack" => zone == ZoneType::Stack,
        _ => true,
    }
}

fn matches_valid_activator(
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

fn matches_valid_target(valid: Option<&str>, target: &Card, source: &Card) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") || v.eq_ignore_ascii_case("Permanent") => true,
        Some(v) if v.eq_ignore_ascii_case("Creature") => target.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => target.id == source.id,
        Some(v)
            if v.eq_ignore_ascii_case("Creature.YouCtrl")
                || v.eq_ignore_ascii_case("Creature.YouControl") =>
        {
            target.is_creature() && target.controller == source.controller
        }
        Some(v) if v.eq_ignore_ascii_case("Creature.OppCtrl") => {
            target.is_creature() && target.controller != source.controller
        }
        _ => true,
    }
}
