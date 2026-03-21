use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::staticability::StaticMode;

pub fn cant_attach(
    cards: &[CardInstance],
    attachment: &CardInstance,
    target: &CardInstance,
    check_sba: bool,
) -> bool {
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantAttach)
        {
            if !matches_valid_card(
                st_ab.params.get(keys::VALID_CARD),
                attachment,
                source,
            ) {
                continue;
            }
            if !matches_valid_card(
                st_ab.params.get(keys::TARGET),
                target,
                source,
            ) {
                continue;
            }
            if let Some(valid_card_to_target) = st_ab.params.get(keys::VALID_CARD_TO_TARGET) {
                if !matches_valid_card_for_target(attachment, valid_card_to_target, target) {
                    continue;
                }
            }
            if (check_sba || !st_ab.params.has(keys::EXCEPTION_SBA))
                && st_ab.params.has(keys::EXCEPTIONS)
                && matches_valid_card(
                    st_ab.params.get(keys::EXCEPTIONS),
                    attachment,
                    source,
                )
            {
                continue;
            }
            return true;
        }
    }
    false
}

fn matches_valid_card(valid: Option<&str>, card: &CardInstance, source: &CardInstance) -> bool {
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
                "Card.IsRemembered" => source.remembered_cards.contains(&card.id),
                "Card.EffectSource" => source.effect_source == Some(card.id),
                "nonLegendary" => !card.type_line.is_legendary(),
                "Legendary" => card.type_line.is_legendary(),
                "YouCtrl" | "YouControl" => card.controller == source.controller,
                "OppCtrl" | "OpponentCtrl" => card.controller != source.controller,
                _ => true,
            })
    })
}

fn matches_valid_card_for_target(card: &CardInstance, valid: &str, target: &CardInstance) -> bool {
    valid.split(',').any(|clause| {
        clause
            .split('+')
            .flat_map(|s| s.split('.'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .all(|tok| match tok {
                "Card" | "Permanent" => true,
                "Creature" => card.is_creature(),
                "Card.Self" => card.id == target.id,
                "nonLegendary" => !target.type_line.is_legendary(),
                "Legendary" => target.type_line.is_legendary(),
                _ => true,
            })
    })
}
