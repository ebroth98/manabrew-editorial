use forge_foundation::ZoneType;

use crate::card::Card;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn blocks_each_combat_if_able(cards: &[Card], creature: &Card) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::MustBlock)
        {
            if matches_valid_creature(st_ab.params.get(keys::VALID_CREATURE), creature, source) {
                return true;
            }
        }
    }
    false
}

pub fn apply_blocks_each_combat_if_able(
    st_ab: &crate::staticability::StaticAbility,
    creature: &Card,
    source: &Card,
) -> bool {
    matches_valid_creature(st_ab.params.get(keys::VALID_CREATURE), creature, source)
}

fn matches_valid_creature(valid: Option<&str>, card: &Card, source: &Card) -> bool {
    match valid {
        None => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        Some(v)
            if v.eq_ignore_ascii_case("Creature.YouCtrl")
                || v.eq_ignore_ascii_case("Creature.YouControl") =>
        {
            card.is_creature() && card.controller == source.controller
        }
        Some(v) if v.eq_ignore_ascii_case("Creature.OppCtrl") => {
            card.is_creature() && card.controller != source.controller
        }
        _ => card.is_creature(),
    }
}
