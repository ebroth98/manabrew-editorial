use crate::card::{valid_filter, CardInstance};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn can_activate(cards: &[CardInstance], card: &CardInstance) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source.static_abilities.iter().filter(|sa| sa.mode == StaticMode::ActivateAbilityAsIfHaste && sa.zones_check(source.zone)) {
            if apply_can_activate_ability(st_ab, card, source) {
                return true;
            }
        }
    }
    false
}

fn apply_can_activate_ability(
    st_ab: &crate::staticability::StaticAbility,
    card: &CardInstance,
    source: &CardInstance,
) -> bool {
    valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_CARD), card, source)
}
