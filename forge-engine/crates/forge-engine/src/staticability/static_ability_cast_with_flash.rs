use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;
use crate::parsing::Params;

pub fn any_with_flash(
    cards: &[Card],
    spell_card: &Card,
    caster: PlayerId,
    spell_abilities: &[String],
) -> bool {
    // Java includes both global static sources and the card itself.
    for source in cards.iter().filter(|c| {
        c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command || c.id == spell_card.id
    }) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CastWithFlash)
        {
            if !matches_valid_card(
                st_ab.params.get(keys::VALID_CARD),
                spell_card,
                source,
            ) {
                continue;
            }
            if !matches_valid_player(
                st_ab.params.get(keys::CASTER),
                caster,
                source.controller,
            ) {
                continue;
            }
            if let Some(valid_sa) = st_ab.params.get(keys::VALID_SA) {
                // "Spell" matches any card being cast as a spell (creatures,
                // sorceries, etc.) — not just cards with explicit SP$ lines.
                // Java treats the inherent spell ability of a card as matching.
                let sa_matches = valid_sa
                    .split(',')
                    .map(str::trim)
                    .any(|tok| tok.eq_ignore_ascii_case("Spell"))
                    || spell_abilities
                        .iter()
                        .any(|line| spell_ability_matches(valid_sa, line));
                if !sa_matches {
                    continue;
                }
            }
            return true;
        }
    }
    false
}

pub fn any_with_flash_needs_info(
    cards: &[Card],
    spell_card: &Card,
    caster: PlayerId,
    spell_abilities: &[String],
) -> bool {
    any_with_flash(cards, spell_card, caster, spell_abilities)
}

pub fn apply_with_flash_needs_info(
    st_ab: &crate::staticability::StaticAbility,
    spell_card: &Card,
    source: &Card,
    caster: PlayerId,
    spell_abilities: &[String],
) -> bool {
    if !matches_valid_card(st_ab.params.get(keys::VALID_CARD), spell_card, source) {
        return false;
    }
    if !matches_valid_player(st_ab.params.get(keys::CASTER), caster, source.controller) {
        return false;
    }
    if let Some(valid_sa) = st_ab.params.get(keys::VALID_SA) {
        let sa_matches = valid_sa
            .split(',')
            .map(str::trim)
            .any(|tok| tok.eq_ignore_ascii_case("Spell"))
            || spell_abilities
                .iter()
                .any(|line| spell_ability_matches(valid_sa, line));
        if !sa_matches {
            return false;
        }
    }
    true
}

pub fn apply_with_flash_ability(
    st_ab: &crate::staticability::StaticAbility,
    spell_card: &Card,
    source: &Card,
    caster: PlayerId,
) -> bool {
    matches_valid_card(st_ab.params.get(keys::VALID_CARD), spell_card, source)
        && matches_valid_player(st_ab.params.get(keys::CASTER), caster, source.controller)
}

fn matches_valid_player(
    valid: Option<&str>,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    valid_filter::matches_valid_player_opt(valid, player, source_controller)
}

fn matches_valid_card(valid: Option<&str>, card: &Card, source: &Card) -> bool {
    valid_filter::matches_valid_card_opt(valid, card, source)
}

fn spell_ability_matches(valid_sa: &str, ability_line: &str) -> bool {
    let params = Params::from_raw(ability_line);
    if !params.has(keys::SP) {
        return false;
    }
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
            "spell" => true,
            "istargeting" => params.has(keys::VALID_TGTS),
            "xcost" => {
                params.get(keys::COST).map(|c| c.contains('X')).unwrap_or(false)
                    || ability_line.contains("X")
            }
            _ => false,
        })
}
