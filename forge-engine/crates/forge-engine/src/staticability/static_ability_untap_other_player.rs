use crate::card::{valid_filter, Card};
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;
use forge_foundation::ZoneType;

pub fn untap(cards: &[Card], card: &Card, player: PlayerId) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::UntapOtherPlayer && sa.zones_check(source.zone))
        {
            if apply_untap_ability(st_ab, card, source, player) {
                return true;
            }
        }
    }
    false
}

pub fn apply_untap_ability(
    st_ab: &crate::staticability::StaticAbility,
    card: &Card,
    source: &Card,
    player: PlayerId,
) -> bool {
    if !valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_CARD), card, source) {
        return false;
    }
    if !valid_filter::matches_valid_player_opt(
        st_ab.params.get(keys::VALID_PLAYER),
        player,
        source.controller,
    ) {
        return false;
    }
    true
}
