use serde::{Deserialize, Serialize};

use crate::cost::{parse_cost, Cost};
use crate::parsing::keys;
use crate::parsing::Params;

/// A parsed activated ability from a card's A: line.
/// Mirrors Java's SpellAbility with AB$ prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivatedAbility {
    /// Index of this ability in the card's abilities list.
    pub ability_index: usize,
    /// The parsed cost to activate.
    pub cost: Cost,
    /// The full raw ability text (for effect resolution reuse).
    pub ability_text: String,
    /// Whether this is a mana ability (resolves without using the stack).
    pub is_mana_ability: bool,
    /// Parsed pipe-delimited parameters.
    pub params: Params,
}

/// Parse an ability string into an ActivatedAbility, if it's an AB$ line.
/// Returns None for SP$/DB$/trigger lines.
///
/// Example AB$ lines:
/// - `"AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}."`
/// - `"AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | ..."`
/// - `"AB$ ChangeZone | Cost$ Sac<1/CARDNAME> | Origin$ Library | ..."`
pub fn parse_activated_ability(raw: &str, index: usize) -> Option<ActivatedAbility> {
    let params = Params::from_raw(raw);

    // Check if any key contains "AB" — the main key is something like "AB" with value "Mana"
    // In practice the format is "AB$ Mana | Cost$ T | ..."
    // After Params::from_raw, we get {"AB": "Mana", "Cost": "T", ...}
    let has_ab = params.has(keys::AB);
    if !has_ab {
        return None;
    }

    // Extract cost
    let cost_str = params.get(keys::COST).unwrap_or("");
    let cost = parse_cost(cost_str);

    // Determine if this is a mana ability:
    // - Effect type is "Mana"
    // - No ValidTgts$ (targeting makes it non-mana)
    // - No loyalty cost
    let ab_type = params.get(keys::AB).unwrap_or("");
    let has_targets = params.has(keys::VALID_TGTS);
    let is_mana_ability = (ab_type.eq_ignore_ascii_case("Mana")
        || ab_type.eq_ignore_ascii_case("ManaReflected"))
        && !has_targets;

    Some(ActivatedAbility {
        ability_index: index,
        cost,
        ability_text: raw.to_string(),
        is_mana_ability,
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_llanowar_elves_mana_ability() {
        let raw = "AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.";
        let ab = parse_activated_ability(raw, 0).unwrap();
        assert!(ab.is_mana_ability);
        assert!(ab.cost.has_tap);
        assert_eq!(ab.params.get(keys::PRODUCED).unwrap(), "G");
    }

    #[test]
    fn parse_prodigal_sorcerer_non_mana() {
        let raw = "AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | SpellDescription$ CARDNAME deals 1 damage to any target.";
        let ab = parse_activated_ability(raw, 0).unwrap();
        assert!(!ab.is_mana_ability);
        assert!(ab.cost.has_tap);
        assert_eq!(ab.params.get(keys::NUM_DMG).unwrap(), "1");
    }

    #[test]
    fn parse_sakura_tribe_elder_sacrifice() {
        let raw = "AB$ ChangeZone | Cost$ Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | SpellDescription$ Search for a basic land.";
        let ab = parse_activated_ability(raw, 0).unwrap();
        assert!(!ab.is_mana_ability);
        assert!(!ab.cost.has_tap);
        assert_eq!(ab.params.get(keys::ORIGIN).unwrap(), "Library");
        assert_eq!(ab.params.get(keys::DESTINATION).unwrap(), "Battlefield");
    }

    #[test]
    fn sp_line_returns_none() {
        let raw = "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ test";
        assert!(parse_activated_ability(raw, 0).is_none());
    }

    #[test]
    fn db_line_returns_none() {
        let raw = "DB$ Draw | Defined$ You | NumCards$ 2";
        assert!(parse_activated_ability(raw, 0).is_none());
    }
}
