use crate::card::valid_filter;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;
use forge_foundation::ZoneType;

pub fn gain_life_radiation(game: &GameState, player: PlayerId) -> bool {
    for card in game
        .cards
        .iter()
        .filter(|c| c.zone.is_static_ability_source())
    {
        for st_ab in card
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::GainLifeRadiation && sa.zones_check(card.zone))
        {
            if valid_filter::matches_valid_player_selector_opt(
                st_ab.params.selector(keys::VALID_PLAYER),
                player,
                card.controller,
            ) {
                return true;
            }
        }
    }
    false
}
