//! Java parity bridge for `StaticAbilityContinuous.java`.
//! Canonical CR613 recomputation lives in `layer.rs`.

use crate::card::CardInstance;
use crate::game::GameState;
use crate::parsing::keys;
use crate::staticability::{Layer, StaticAbility};

pub fn apply_continuous_ability(
    st_ab: &StaticAbility,
    source: &CardInstance,
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

pub fn resolve(st_ab: &StaticAbility, source: &CardInstance, game: &GameState) {
    let _ = run(st_ab, source, game);
}

pub fn can_play(st_ab: &StaticAbility, card: &CardInstance, _game: &GameState) -> bool {
    if !st_ab.zones_check(card.zone) {
        return false;
    }
    if !matches!(st_ab.params.get("MayPlay"), Some(v) if v.eq_ignore_ascii_case("True")) {
        return false;
    }
    crate::card::valid_filter::matches_valid_card_opt(st_ab.params.get(keys::AFFECTED), card, card)
}

pub fn run(st_ab: &StaticAbility, source: &CardInstance, game: &GameState) -> bool {
    st_ab.check_conditions(source, game)
}
