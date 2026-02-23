//! Card property matching for targeting and filtering.
//!
//! Mirrors Java's `CardProperty.cardHasProperty()` — evaluates whether a card
//! matches a property string used in `ValidTgts$` filters (e.g. "nonBlack",
//! "OppCtrl", "YouCtrl").

use forge_foundation::ColorSet;

use crate::card::CardInstance;
use crate::ids::PlayerId;

/// Check if a card matches a compound filter string.
/// Supports color filters ("nonBlack"), controller filters ("OppCtrl", "YouCtrl"),
/// and combined dot-separated filters (e.g. "OppCtrl.nonBlack").
/// Mirrors Java's `CardProperty.cardHasProperty()` with dot-separated qualifiers.
pub fn card_has_property(card: &CardInstance, filter: &str, source_controller: PlayerId) -> bool {
    // Handle dot-separated compound filters (e.g. "OppCtrl.nonBlack")
    for part in filter.split('.') {
        if !matches_single_property(card, part, source_controller) {
            return false;
        }
    }
    true
}

/// Match a single property qualifier against a card.
/// Mirrors individual property checks in Java's `CardProperty.cardHasProperty()`.
fn matches_single_property(
    card: &CardInstance,
    property: &str,
    source_controller: PlayerId,
) -> bool {
    match property {
        "OppCtrl" => card.controller != source_controller,
        "YouCtrl" => card.controller == source_controller,
        "Other" => true, // "Other" means "not self" — handled at call site
        _ => {
            let lower = property.to_ascii_lowercase();
            if let Some(color_name) = lower.strip_prefix("non") {
                let excluded = ColorSet::from_names(color_name);
                !card.color.shares_color_with(excluded)
            } else {
                // No recognized property — match everything
                true
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

        let black_creature = CardInstance::new(
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
        let green_creature = CardInstance::new(
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

        let mut card = CardInstance::new(
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

        let mut card = CardInstance::new(
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
}
