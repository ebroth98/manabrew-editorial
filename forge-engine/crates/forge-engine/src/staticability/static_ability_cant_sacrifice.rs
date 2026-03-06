use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_sacrifice(
    cards: &[CardInstance],
    card: &CardInstance,
    cause: Option<&SpellAbility>,
    is_cost: bool,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantSacrifice)
        {
            if let Some(for_cost) = st_ab.params.get("ForCost") {
                if for_cost.eq_ignore_ascii_case("True") != is_cost {
                    continue;
                }
            }
            if !matches_valid_card(
                st_ab.params.get("ValidCard").map(String::as_str),
                card,
                source,
            ) {
                continue;
            }
            if st_ab.params.contains_key("ValidCause") && cause.is_none() {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Card") || v.eq_ignore_ascii_case("Permanent") => true,
        Some(v) if v.eq_ignore_ascii_case("Card.Self") => card.id == source.id,
        Some(v) if v.eq_ignore_ascii_case("Creature") => card.is_creature(),
        Some(v)
            if v.eq_ignore_ascii_case("Creature.YouCtrl")
                || v.eq_ignore_ascii_case("Creature.YouControl") =>
        {
            card.is_creature() && card.controller == source.controller
        }
        Some(v)
            if v.eq_ignore_ascii_case("Creature.OppCtrl")
                || v.eq_ignore_ascii_case("Creature.OpponentCtrl") =>
        {
            card.is_creature() && card.controller != source.controller
        }
        _ => true,
    }
}
