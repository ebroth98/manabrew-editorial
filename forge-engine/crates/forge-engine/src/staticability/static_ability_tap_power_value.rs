use crate::card::{valid_filter, Card};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

/// Check if a card should use toughness as its tap power value.
///
/// Mirrors Java's `StaticAbilityTapPowerValue.withToughness(Card, CardTraitBase)`.
/// The `sa` parameter mirrors Java's `CardTraitBase ctb` for ValidSA matching.
pub fn with_toughness(
    cards: &[Card],
    card: &Card,
    _sa: Option<&SpellAbility>,
) -> bool {
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source.static_abilities.iter().filter(|s| s.mode == StaticMode::TapPowerValue && s.zones_check(source.zone)) {
            // Value$ must equal "Toughness" (case-insensitive to match Java .equals behavior)
            match st_ab.params.get(keys::VALUE) {
                Some(val) if val.eq_ignore_ascii_case("Toughness") => {}
                _ => continue,
            }

            // ValidCard$
            if !valid_filter::matches_valid_card_opt(
                st_ab.params.get(keys::VALID_CARD),
                card,
                source,
            ) {
                continue;
            }

            // ValidSA$ — Java checks stAb.matchesValidParam("ValidSA", ctb).
            // TODO: implement full ValidSA matching once CardTraitBase validation is ported.
            // For now we permissively pass (matches Java behavior when param is absent).

            return true;
        }
    }
    false
}

/// Get the modifier for tap power value.
///
/// Mirrors Java's `StaticAbilityTapPowerValue.getMod()`.
/// The `sa` parameter mirrors Java's `CardTraitBase ctb` for ValidSA matching.
pub fn get_mod(
    cards: &[Card],
    card: &Card,
    _sa: Option<&SpellAbility>,
) -> i32 {
    let mut total = 0;
    for source in cards.iter().filter(|c| c.zone.is_static_ability_source()) {
        for st_ab in source.static_abilities.iter().filter(|s| s.mode == StaticMode::TapPowerValue && s.zones_check(source.zone)) {
            // ValidCard$
            if !valid_filter::matches_valid_card_opt(
                st_ab.params.get(keys::VALID_CARD),
                card,
                source,
            ) {
                continue;
            }

            // ValidSA$ — same TODO as above

            if let Some(val) = st_ab.params.get(keys::VALUE) {
                total += val.parse::<i32>().unwrap_or(0);
            }
        }
    }
    total
}
