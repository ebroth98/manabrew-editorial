//! Card property matching for targeting and filtering.
//!
//! Mirrors Java's `CardProperty.cardHasProperty()` — evaluates whether a card
//! matches a property string used in `ValidTgts$` filters (e.g. "nonBlack",
//! "OppCtrl", "YouCtrl").

use forge_foundation::ColorSet;

use crate::card::filter_constants as fc;
use crate::card::Card;
use crate::ids::PlayerId;

/// Check if a card matches a compound filter string.
/// Supports color filters ("nonBlack"), controller filters ("OppCtrl", "YouCtrl"),
/// and combined dot-separated filters (e.g. "OppCtrl.nonBlack").
/// Mirrors Java's `CardProperty.cardHasProperty()` with dot-separated qualifiers.
pub fn card_has_property(card: &Card, filter: &str, source_controller: PlayerId) -> bool {
    // Handle compound filters separated by '.' or '+' (e.g. "OppCtrl.nonBlack", "nonLand+OppCtrl")
    // Both separators mean AND — all parts must match.
    for part in filter.split(|c| c == '.' || c == '+') {
        if !matches_single_property(card, part, source_controller) {
            return false;
        }
    }
    true
}

/// Match a single property qualifier against a card.
/// Mirrors individual property checks in Java's `CardProperty.cardHasProperty()`.
fn matches_single_property(
    card: &Card,
    property: &str,
    source_controller: PlayerId,
) -> bool {
    match property {
        // Inclusive type checks (mirrors Java Card.isValid type token before dot).
        fc::CARD | "card" => true,
        fc::PERMANENT => card.is_permanent(),
        fc::CREATURE => card.is_creature(),
        fc::LAND => card.is_land(),
        fc::INSTANT => card.type_line.is_instant(),
        fc::SORCERY => card.type_line.is_sorcery(),
        fc::ARTIFACT => card.type_line.is_artifact(),
        fc::ENCHANTMENT => card.type_line.is_enchantment(),
        fc::PLANESWALKER => card.type_line.is_planeswalker(),
        fc::OPP_CTRL => card.controller != source_controller,
        fc::YOU_CTRL => card.controller == source_controller,
        fc::YOU_DONT_CTRL => card.controller != source_controller,
        // Combat qualifier used by scripts such as Stalking Leonin
        // (`ValidTgts$ Creature.attackingYou`): card must currently be
        // attacking the source controller.
        "attackingYou" => card.attacking_player == Some(source_controller),
        fc::OTHER => true, // "Other" means "not self" — handled at call site
        // Type-based filters
        fc::NON_LAND => !card.type_line.is_land(),
        fc::NON_CREATURE => !card.is_creature(),
        fc::NON_ARTIFACT => !card.type_line.is_artifact(),
        _ => {
            let lower = property.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("cmcge") {
                if let Ok(n) = rest.parse::<i32>() {
                    return card.mana_cost.cmc() >= n;
                }
                return false;
            }
            if let Some(rest) = lower.strip_prefix("cmcgt") {
                if let Ok(n) = rest.parse::<i32>() {
                    return card.mana_cost.cmc() > n;
                }
                return false;
            }
            if let Some(rest) = lower.strip_prefix("cmcle") {
                if let Ok(n) = rest.parse::<i32>() {
                    return card.mana_cost.cmc() <= n;
                }
                return false;
            }
            if let Some(rest) = lower.strip_prefix("cmclt") {
                if let Ok(n) = rest.parse::<i32>() {
                    return card.mana_cost.cmc() < n;
                }
                return false;
            }
            if let Some(rest) = lower.strip_prefix("cmceq") {
                if let Ok(n) = rest.parse::<i32>() {
                    return card.mana_cost.cmc() == n;
                }
                return false;
            }
            if let Some(color_name) = lower.strip_prefix("non") {
                let excluded = ColorSet::from_names(color_name);
                !card.color.shares_color_with(excluded)
            } else if let Some(keyword) = property.strip_prefix("without") {
                !card.has_keyword(keyword)
            } else if let Some(keyword) = property.strip_prefix("with") {
                card.has_keyword(keyword)
            } else {
                // Check if it's a creature subtype (Wall, Zombie, Elf, etc.).
                // Mirrors Java's CardProperty.cardHasProperty() subtype matching.
                card.type_line.has_subtype(property)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::ManaCost;

    #[test]
    fn non_black_filter() {
        use crate::ids::CardId;

        let black_creature = Card::new(
            CardId(0),
            "Doom".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Zombie"),
            ManaCost::parse("1 B"),
            ColorSet::BLACK,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let green_creature = Card::new(
            CardId(1),
            "Bear".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let caster = PlayerId(1);
        assert!(!card_has_property(&black_creature, "nonBlack", caster));
        assert!(card_has_property(&green_creature, "nonBlack", caster));
    }

    #[test]
    fn opp_ctrl_filter() {
        use crate::ids::CardId;

        let mut card = Card::new(
            CardId(0),
            "Bear".to_string(),
            PlayerId(1),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        card.controller = PlayerId(1);
        let caster = PlayerId(0);
        assert!(card_has_property(&card, "OppCtrl", caster));
        assert!(!card_has_property(&card, "YouCtrl", caster));
    }

    #[test]
    fn compound_filter() {
        use crate::ids::CardId;

        let mut card = Card::new(
            CardId(0),
            "Bear".to_string(),
            PlayerId(1),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        card.controller = PlayerId(1);
        let caster = PlayerId(0);
        // OppCtrl.nonBlack → opponent controls + not black → true for green creature
        assert!(card_has_property(&card, "OppCtrl.nonBlack", caster));
    }

    #[test]
    fn creature_you_ctrl_requires_creature_type() {
        use crate::ids::CardId;

        let sorcery = Card::new(
            CardId(2),
            "Innocent Blood".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Sorcery"),
            ManaCost::parse("B"),
            ColorSet::BLACK,
            None,
            None,
            vec![],
            vec![],
        );

        assert!(!card_has_property(
            &sorcery,
            "Creature.YouCtrl",
            PlayerId(0)
        ));
    }

    #[test]
    fn cmc_comparators() {
        use crate::ids::CardId;

        let creature = Card::new(
            CardId(3),
            "Hill Giant".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Giant"),
            ManaCost::parse("3 R"),
            ColorSet::RED,
            Some(3),
            Some(3),
            vec![],
            vec![],
        );

        assert!(card_has_property(&creature, "cmcGE3", PlayerId(0)));
        assert!(card_has_property(&creature, "cmcGT2", PlayerId(0)));
        assert!(card_has_property(&creature, "cmcLE4", PlayerId(0)));
        assert!(card_has_property(&creature, "cmcLT5", PlayerId(0)));
        assert!(card_has_property(&creature, "cmcEQ4", PlayerId(0)));
        assert!(!card_has_property(&creature, "cmcGE5", PlayerId(0)));
        assert!(!card_has_property(&creature, "cmcEQ3", PlayerId(0)));
    }
}
