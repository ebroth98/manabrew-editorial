use forge_foundation::ZoneType;

use crate::card::Card;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn ignore_legend_rule(cards: &[Card], card: &Card) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::IgnoreLegendRule)
        {
            if !matches_valid_card(st_ab.params.get(keys::VALID_CARD), card, source) {
                continue;
            }
            if !is_present_condition_met(cards, st_ab, source) {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &Card, source: &Card) -> bool {
    let Some(expr) = valid else {
        return true;
    };
    expr.split(',').any(|clause| {
        clause
            .split('+')
            .flat_map(|s| s.split('.'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .all(|tok| match tok {
                "Card" | "Permanent" => true,
                "Creature" => card.is_creature(),
                "Card.Self" => card.id == source.id,
                "namedBrothers Yamazaki" => card.card_name == "Brothers Yamazaki",
                "YouCtrl" | "YouControl" => card.controller == source.controller,
                "OppCtrl" | "OpponentCtrl" => card.controller != source.controller,
                _ => true,
            })
    })
}

fn is_present_condition_met(
    cards: &[Card],
    st_ab: &crate::staticability::StaticAbility,
    source: &Card,
) -> bool {
    let Some(is_present) = st_ab.params.get(keys::IS_PRESENT) else {
        return true;
    };
    let count = cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .filter(|c| matches_valid_card(Some(is_present), c, source))
        .count() as i32;
    let cmp = st_ab.params.get(keys::PRESENT_COMPARE).unwrap_or("GE1");
    match cmp {
        "EQ2" => count == 2,
        "EQ1" => count == 1,
        "GE1" => count >= 1,
        _ => count >= 1,
    }
}
