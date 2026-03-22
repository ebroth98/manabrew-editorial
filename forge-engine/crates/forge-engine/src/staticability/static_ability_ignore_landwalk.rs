use forge_foundation::ZoneType;
use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn ignore_land_walk(
    cards: &[Card],
    attacker: &Card,
    blocker: &Card,
    keyword: &str,
) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source.static_abilities.iter().filter(|sa| sa.mode == StaticMode::IgnoreLandwalk && sa.zones_check(source.zone)) {
            if apply_ignore_landwalk(st_ab, attacker, blocker, keyword, source) {
                return true;
            }
        }
    }
    false
}

/// Alias for `apply_ignore_landwalk` — matches Java naming.
pub fn ignore_land_walk_ability(
    st_ab: &crate::staticability::StaticAbility,
    attacker: &Card,
    blocker: &Card,
    keyword: &str,
    source: &Card,
) -> bool {
    apply_ignore_landwalk(st_ab, attacker, blocker, keyword, source)
}

fn apply_ignore_landwalk(
    st_ab: &crate::staticability::StaticAbility,
    attacker: &Card,
    blocker: &Card,
    keyword: &str,
    source: &Card,
) -> bool {
    if !valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_ATTACKER), attacker, source) {
        return false;
    }
    if !valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_BLOCKER), blocker, source) {
        return false;
    }
    if let Some(valid_kw) = st_ab.params.get(keys::VALID_KEYWORD) {
        if !valid_kw.eq_ignore_ascii_case(keyword) {
            return false;
        }
    }
    true
}
