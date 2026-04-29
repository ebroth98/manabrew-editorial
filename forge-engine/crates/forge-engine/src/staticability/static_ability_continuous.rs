//! Java parity bridge for `StaticAbilityContinuous.java`.
//! Canonical CR613 recomputation lives in `layer.rs`.

use crate::card::Card;
use crate::game::GameState;
use crate::staticability::{Layer, StaticAbility};

pub fn apply_continuous_ability(
    st_ab: &StaticAbility,
    source: &Card,
    game: &mut GameState,
    layer: Layer,
) {
    if !crate::staticability::layer::classify_static_layers(st_ab).contains(&layer) {
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
    if !st_ab.ir.may_play {
        return false;
    }
    // Check AffectedZone$ — the zone where the affected cards must be.
    // Default is Hand if not specified (normal MayPlay like casting from hand).
    if !st_ab.ir.affected_zones.is_empty() {
        if !st_ab.ir.affected_zones.contains(&card.zone) {
            return false;
        }
    } else if card.zone != forge_foundation::ZoneType::Hand {
        return false;
    }
    crate::card::valid_filter::matches_valid_card_selector_opt(
        st_ab.ir.affected.as_ref(),
        card,
        source,
    )
}

/// If a `MayPlay$ True` static on `source` grants `card` permission to be
/// cast and also defines `MayPlayAltManaCost$`, return that alt cost string.
/// Mirrors Java's `GameActionUtil.canPlayCardMayPlay` reading
/// `getMayPlayAltManaCost()` for the granting static.
pub fn may_play_alt_mana_cost(
    st_ab: &StaticAbility,
    source: &Card,
    card: &Card,
    game: &GameState,
) -> Option<String> {
    if !can_play(st_ab, source, card, game) {
        return None;
    }
    st_ab.ir.may_play_alt_mana_cost.clone()
}

pub fn run(st_ab: &StaticAbility, source: &Card, game: &GameState) -> bool {
    st_ab.check_conditions(source, game)
}
