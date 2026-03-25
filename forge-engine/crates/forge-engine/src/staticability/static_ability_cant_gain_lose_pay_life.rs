use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_gain_life(game: &GameState, player: PlayerId) -> bool {
    any_common(
        game,
        player,
        &[StaticMode::CantGainLife, StaticMode::CantChangeLife],
        None,
        false,
    )
}

pub fn any_cant_gain_life(game: &GameState, player: PlayerId) -> bool {
    cant_gain_life(game, player)
}

pub fn cant_lose_life(game: &GameState, player: PlayerId) -> bool {
    any_common(
        game,
        player,
        &[StaticMode::CantLoseLife, StaticMode::CantChangeLife],
        None,
        false,
    )
}

pub fn any_cant_lose_life(game: &GameState, player: PlayerId) -> bool {
    cant_lose_life(game, player)
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
            StaticMode::CantPayLife,
            StaticMode::CantLoseLife,
            StaticMode::CantChangeLife,
        ],
        cause,
        is_cost,
    )
}

pub fn any_cant_pay_life(
    game: &GameState,
    player: PlayerId,
    is_cost: bool,
    cause: Option<&SpellAbility>,
) -> bool {
    cant_pay_life(game, player, is_cost, cause)
}

pub fn apply_common_ability(
    st_ab: &crate::staticability::StaticAbility,
    source_controller: PlayerId,
    player: PlayerId,
    is_cost: bool,
) -> bool {
    if let Some(for_cost) = st_ab.params.get(keys::FOR_COST) {
        if for_cost.eq_ignore_ascii_case("True") != is_cost {
            return false;
        }
    }
    matches_valid_player(
        st_ab.params.get(keys::VALID_PLAYER),
        player,
        source_controller,
    )
}

fn any_common(
    game: &GameState,
    player: PlayerId,
    modes: &[StaticMode],
    _cause: Option<&SpellAbility>,
    is_cost: bool,
) -> bool {
    for card in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
    {
        for st_ab in &card.static_abilities {
            if !modes.iter().any(|m| st_ab.mode == *m) {
                continue;
            }
            if let Some(for_cost) = st_ab.params.get(keys::FOR_COST) {
                if for_cost.eq_ignore_ascii_case("True") != is_cost {
                    continue;
                }
            }
            if !matches_valid_player(
                st_ab.params.get(keys::VALID_PLAYER),
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
