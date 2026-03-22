use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn must_attack(cards: &[CardInstance], attacker: &CardInstance) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::MustAttack)
        {
            if matches_valid_creature(
                st_ab.params.get(keys::VALID_CREATURE),
                attacker,
                source,
            ) {
                return true;
            }
        }
    }
    false
}

pub fn entities_must_attack(cards: &[CardInstance], attacker: &CardInstance) -> bool {
    must_attack(cards, attacker)
}

fn matches_valid_creature(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
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
