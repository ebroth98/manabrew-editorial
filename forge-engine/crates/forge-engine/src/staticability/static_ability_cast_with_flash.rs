use forge_foundation::ZoneType;

use crate::card::{valid_filter, CardInstance};
use crate::ids::PlayerId;
use crate::staticability::StaticMode;
use crate::trigger::parse_pipe_params;

pub fn any_with_flash(
    cards: &[CardInstance],
    spell_card: &CardInstance,
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
                st_ab.params.get("ValidCard").map(String::as_str),
                spell_card,
                source,
            ) {
                continue;
            }
            if !matches_valid_player(
                st_ab.params.get("Caster").map(String::as_str),
                caster,
                source.controller,
            ) {
                continue;
            }
            if let Some(valid_sa) = st_ab.params.get("ValidSA") {
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

fn matches_valid_player(
    valid: Option<&str>,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    valid_filter::matches_valid_player_opt(valid, player, source_controller)
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    valid_filter::matches_valid_card_opt(valid, card, source)
}

fn spell_ability_matches(valid_sa: &str, ability_line: &str) -> bool {
    let params = parse_pipe_params(ability_line);
    if !params.contains_key("SP") {
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
            "istargeting" => params.contains_key("ValidTgts"),
            "xcost" => {
                params.get("Cost").map(|c| c.contains('X')).unwrap_or(false)
                    || ability_line.contains("X")
            }
            _ => false,
        })
}
