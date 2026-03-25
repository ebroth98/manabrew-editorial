//! AnimateEffectBase — abstract base for animate effects.
//!
//! Mirrors Java's `AnimateEffectBase.java`.
//! Provides shared logic for `AnimateEffect` and `AnimateAllEffect`
//! that handles setting power/toughness, types, colors, and keywords.

use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Parsed animate parameters from a spell ability.
/// Captures the fields that animate effects need to apply.
#[derive(Debug, Clone, Default)]
pub struct AnimateParams {
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub add_types: Vec<String>,
    pub add_keywords: Vec<String>,
    pub colors: Option<Vec<String>>,
    pub overwrite_types: bool,
}

/// Parse shared animate parameters from a spell ability.
/// Used by both `animate_effect` and `animate_all_effect`.
pub fn parse_animate_params(sa: &SpellAbility) -> AnimateParams {
    let mut params = AnimateParams::default();

    if let Some(p) = sa.params.get(keys::POWER) {
        params.power = p.parse::<i32>().ok();
    }
    if let Some(t) = sa.params.get(keys::TOUGHNESS) {
        params.toughness = t.parse::<i32>().ok();
    }
    if let Some(types) = sa.params.get("Types") {
        params.add_types = types.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(kws) = sa.params.get(keys::KEYWORDS) {
        params.add_keywords = kws.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(colors) = sa.params.get("Colors") {
        params.colors = Some(colors.split(',').map(|s| s.trim().to_string()).collect());
    }
    params.overwrite_types = sa
        .params
        .get("OverwriteTypes")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    params
}
