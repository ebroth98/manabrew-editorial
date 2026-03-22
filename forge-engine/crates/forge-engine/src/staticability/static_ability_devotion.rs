use forge_foundation::ZoneType;
use crate::card::valid_filter;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn get_devotion_mod(game: &GameState, player: PlayerId) -> i32 {
    let mut total = 0;
    for card in game.cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in card.static_abilities.iter().filter(|sa| sa.mode == StaticMode::Devotion && sa.zones_check(card.zone)) {
            if !valid_filter::matches_valid_player_opt(
                st_ab.params.get(keys::VALID_PLAYER),
                player,
                card.controller,
            ) {
                continue;
            }
            let val = st_ab.params.get(keys::VALUE)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(1);
            total += val;
        }
    }
    total
}
