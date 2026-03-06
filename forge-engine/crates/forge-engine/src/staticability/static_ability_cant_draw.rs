use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn can_draw_amount(game: &GameState, player: PlayerId, start_amount: i32) -> i32 {
    if start_amount <= 0 {
        return 0;
    }
    let mut amount = start_amount;
    for card in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
    {
        for st_ab in card
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantDraw)
        {
            let valid_player = st_ab.params.get("ValidPlayer").map(String::as_str);
            if !matches_valid_player(valid_player, player, card.controller) {
                continue;
            }
            let limit = st_ab
                .params
                .get("DrawLimit")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            let drawn = game.player(player).drawn_this_turn;
            amount = amount.min((limit - drawn).max(0));
        }
    }
    amount
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
