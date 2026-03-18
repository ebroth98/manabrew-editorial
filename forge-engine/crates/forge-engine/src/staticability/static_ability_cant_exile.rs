use forge_foundation::ZoneType;

use crate::card::{valid_filter, CardInstance};
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_exile(
    cards: &[CardInstance],
    card: &CardInstance,
    cause: Option<&SpellAbility>,
    is_cost: bool,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantExile)
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
            if !matches_valid_cause(st_ab.params.get("ValidCause").map(String::as_str), cause) {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
    valid_filter::matches_valid_card_opt(valid, card, source)
}

fn matches_valid_cause(valid: Option<&str>, cause: Option<&SpellAbility>) -> bool {
    let Some(valid) = valid else {
        return true;
    };
    let Some(cause) = cause else {
        return false;
    };

    valid.split(',').any(|token| {
        let token = token.trim();
        if token.is_empty() {
            return false;
        }

        let mut segments = token.split('.');
        let head = segments.next().unwrap_or("");
        let base_ok = if head.eq_ignore_ascii_case("SpellAbility") {
            true
        } else if head.eq_ignore_ascii_case("Spell") {
            cause.is_spell
        } else if head.eq_ignore_ascii_case("Ability") {
            !cause.is_spell
        } else {
            false
        };
        if !base_ok {
            return false;
        }

        for qualifier in segments {
            let q = qualifier.trim();
            if q.eq_ignore_ascii_case("EffectSource") && !cause.params.contains_key("EffectSource")
            {
                return false;
            }
            if q.eq_ignore_ascii_case("!EffectSource") && cause.params.contains_key("EffectSource")
            {
                return false;
            }
        }
        true
    })
}
