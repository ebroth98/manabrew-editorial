use forge_foundation::ZoneType;

use crate::card::Card;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn ignore_hexproof(cards: &[Card], target: &Card, activator: PlayerId) -> bool {
    any_ignore(cards, target, activator, StaticMode::IgnoreHexproof)
}

pub fn ignore_shroud(cards: &[Card], target: &Card, activator: PlayerId) -> bool {
    any_ignore(cards, target, activator, StaticMode::IgnoreShroud)
}

fn any_ignore(
    cards: &[Card],
    target: &Card,
    activator: PlayerId,
    mode: StaticMode,
) -> bool {
    for source in cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for st_ab in source.static_abilities.iter().filter(|sa| sa.mode == mode) {
            if !matches_valid_player(
                st_ab.params.get(keys::ACTIVATOR),
                activator,
                source.controller,
                source,
            ) {
                continue;
            }
            if !matches_valid_entity(
                st_ab.params.get(keys::VALID_ENTITY),
                target,
                source,
            ) {
                continue;
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
    source: &Card,
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
        Some(v) if v.eq_ignore_ascii_case("Player.IsRemembered") => {
            source.remembered_players.contains(&player)
        }
        _ => true,
    }
}

fn matches_valid_entity(valid: Option<&str>, target: &Card, source: &Card) -> bool {
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
                "Card" | "Permanent" => true,
                "Creature" => target.is_creature(),
                "Card.Self" => target.id == source.id,
                "Card.EffectSource" => source.effect_source == Some(target.id),
                "Card.IsRemembered" => source.remembered_cards.contains(&target.id),
                "YouCtrl" | "YouControl" => target.controller == source.controller,
                "OppCtrl" | "OpponentCtrl" => target.controller != source.controller,
                _ => true,
            })
    })
}
