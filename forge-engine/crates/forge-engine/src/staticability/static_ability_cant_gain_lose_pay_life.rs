use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_gain_life(game: &GameState, player: PlayerId) -> bool {
    any_common(game, player, &[StaticMode::Other("CantGainLife".to_string()), StaticMode::Other("CantChangeLife".to_string())], None, false)
}

pub fn cant_lose_life(game: &GameState, player: PlayerId) -> bool {
    any_common(game, player, &[StaticMode::Other("CantLoseLife".to_string()), StaticMode::Other("CantChangeLife".to_string())], None, false)
}

pub fn cant_pay_life(
    game: &GameState,
    player: PlayerId,
    is_cost: bool,
    cause: Option<&SpellAbility>,
) -> bool {
    any_common(
        game,
        player,
        &[
            StaticMode::Other("CantPayLife".to_string()),
            StaticMode::Other("CantLoseLife".to_string()),
            StaticMode::Other("CantChangeLife".to_string()),
        ],
        cause,
        is_cost,
    )
}

fn any_common(
    game: &GameState,
    player: PlayerId,
    modes: &[StaticMode],
    _cause: Option<&SpellAbility>,
    is_cost: bool,
) -> bool {
    for card in game.cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in &card.static_abilities {
            if !modes.iter().any(|m| match (m, &st_ab.mode) {
                (StaticMode::Other(a), StaticMode::Other(b)) => a.eq_ignore_ascii_case(b),
                _ => false,
            }) {
                continue;
            }
            if let Some(for_cost) = st_ab.params.get("ForCost") {
                if for_cost.eq_ignore_ascii_case("True") != is_cost {
                    continue;
                }
            }
            if !matches_valid_player(
                st_ab.params.get("ValidPlayer").map(String::as_str),
                player,
                card.controller,
            ) {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_player(valid: Option<&str>, player: PlayerId, source_controller: PlayerId) -> bool {
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
