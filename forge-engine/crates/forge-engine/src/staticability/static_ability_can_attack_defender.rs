use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn can_attack_defender(cards: &[Card], card: &Card, defender: PlayerId) -> bool {
    for source in cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CanAttackDefender)
        {
            if !matches_valid_card(st_ab.params.selector(keys::VALID_CARD), card, source) {
                continue;
            }
            if !matches_valid_attacked(
                st_ab.params.get(keys::VALID_ATTACKED),
                defender,
                source.controller,
            ) {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(
    valid: Option<&crate::parsing::CompiledSelector>,
    card: &Card,
    source: &Card,
) -> bool {
    valid_filter::matches_valid_card_selector_opt(valid, card, source)
}

fn matches_valid_attacked(
    valid: Option<&str>,
    defender: PlayerId,
    source_controller: PlayerId,
) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Player") => true,
        Some(v) if v.eq_ignore_ascii_case("You") || v.eq_ignore_ascii_case("YouCtrl") => {
            defender == source_controller
        }
        Some(v) if v.eq_ignore_ascii_case("Opponent") || v.eq_ignore_ascii_case("OppCtrl") => {
            defender != source_controller
        }
        _ => true,
    }
}
