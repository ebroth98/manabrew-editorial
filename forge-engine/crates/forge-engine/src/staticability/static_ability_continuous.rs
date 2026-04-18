//! Java parity bridge for `StaticAbilityContinuous.java`.
//! Canonical CR613 recomputation lives in `layer.rs`.

use crate::card::Card;
use crate::game::GameState;
use crate::parsing::keys;
use crate::staticability::{Layer, StaticAbility};

pub fn apply_continuous_ability(
    st_ab: &StaticAbility,
    source: &Card,
    game: &mut GameState,
    layer: Layer,
) {
    if st_ab.continuous_layer() != Some(layer) {
        return;
    }
    if !st_ab.check_conditions(source, game) {
        return;
    }
    crate::staticability::layer::apply_continuous_effects(game);
}

pub fn resolve(st_ab: &StaticAbility, source: &Card, game: &GameState) {
    let _ = run(st_ab, source, game);
}

pub fn can_play(st_ab: &StaticAbility, source: &Card, card: &Card, _game: &GameState) -> bool {
    if !matches!(st_ab.params.get("MayPlay"), Some(v) if v.eq_ignore_ascii_case("True")) {
        return false;
    }
    // Check AffectedZone$ — the zone where the affected cards must be.
    // Default is Hand if not specified (normal MayPlay like casting from hand).
    if let Some(affected_zone) = st_ab.params.get(keys::AFFECTED_ZONE) {
        let zones: Vec<forge_foundation::ZoneType> = affected_zone
            .split(',')
            .filter_map(|z| forge_foundation::ZoneType::from_str_compat(z.trim()))
            .collect();
        if !zones.is_empty() && !zones.contains(&card.zone) {
            return false;
        }
    } else if card.zone != forge_foundation::ZoneType::Hand {
        return false;
    }
    crate::card::valid_filter::matches_valid_card_opt(
        st_ab.params.get(keys::AFFECTED),
        card,
        source,
    )
}

pub fn run(st_ab: &StaticAbility, source: &Card, game: &GameState) -> bool {
    st_ab.check_conditions(source, game)
}
