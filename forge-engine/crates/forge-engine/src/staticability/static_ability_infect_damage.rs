use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn is_infect_damage(
    game: &GameState,
    cards: &[CardInstance],
    target: PlayerId,
    source_controller: PlayerId,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::InfectDamage)
        {
            if !condition_matches(game, source, st_ab) {
                continue;
            }
            let valid = st_ab.params.get("ValidTarget").map(String::as_str);
            if matches_valid_player(valid, target, source_controller) {
                return true;
            }
        }
    }
    false
}

fn condition_matches(
    game: &GameState,
    source: &CardInstance,
    st_ab: &crate::staticability::StaticAbility,
) -> bool {
    let Some(check_svar) = st_ab.params.get("CheckSVar") else {
        return true;
    };
    let Some(compare) = st_ab.params.get("SVarCompare") else {
        return true;
    };
    let Some(expr) = source.svars.get(check_svar) else {
        return true;
    };
    // Only support the pattern needed by Phyrexian Unlife.
    let value = if expr == "Count$YourLifeTotal" {
        game.player(source.controller).life
    } else {
        return true;
    };
    if let Some(n) = compare
        .strip_prefix("LE")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value <= n;
    }
    if let Some(n) = compare
        .strip_prefix("LT")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value < n;
    }
    if let Some(n) = compare
        .strip_prefix("GE")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value >= n;
    }
    if let Some(n) = compare
        .strip_prefix("GT")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value > n;
    }
    if let Some(n) = compare
        .strip_prefix("EQ")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value == n;
    }
    true
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
