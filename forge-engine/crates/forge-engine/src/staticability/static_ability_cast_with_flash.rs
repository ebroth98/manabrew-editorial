use forge_foundation::ZoneType;

use crate::card::CardInstance;
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
                if !spell_abilities
                    .iter()
                    .any(|line| spell_ability_matches(valid_sa, line))
                {
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

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") => true,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Nonland") => !card.is_land(),
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        _ => true,
    }
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
