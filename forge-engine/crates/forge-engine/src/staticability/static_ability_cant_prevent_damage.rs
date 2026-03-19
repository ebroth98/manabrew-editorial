use forge_foundation::ZoneType;

use crate::card::{valid_filter, CardInstance};
use crate::ids::CardId;
use crate::staticability::StaticMode;

/// Check whether damage from `source_id` cannot be prevented.
/// Mirrors Java's StaticAbilityCantPreventDamage.cantPreventDamage().
pub fn cant_prevent_damage(cards: &[CardInstance], source_id: CardId, is_combat: bool) -> bool {
    let source_card = &cards[source_id.index()];

    for static_source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in static_source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPreventDamage)
        {
            if applies(st_ab, source_card, static_source, is_combat) {
                return true;
            }
        }
    }

    // Java also considers the damage source itself as a potential host.
    if source_card.zone != ZoneType::Battlefield {
        for st_ab in source_card
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantPreventDamage)
        {
            if applies(st_ab, source_card, source_card, is_combat) {
                return true;
            }
        }
    }

    false
}

fn applies(
    st_ab: &crate::staticability::static_ability::StaticAbility,
    damage_source: &CardInstance,
    host: &CardInstance,
    is_combat: bool,
) -> bool {
    if let Some(flag) = st_ab.params.get("IsCombat") {
        let required = flag.eq_ignore_ascii_case("True");
        if required != is_combat {
            return false;
        }
    }

    valid_filter::matches_valid_card_opt(
        st_ab.params.get("ValidSource").map(String::as_str),
        damage_source,
        host,
    )
}

