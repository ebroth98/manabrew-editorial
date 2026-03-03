use serde::{Deserialize, Serialize};

/// Alternative costs for casting spells.
/// Mirrors Java's `AlternativeCost.java` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlternativeCost {
    /// Cast from graveyard for flashback cost; exile after resolution.
    Flashback,
    /// Cast creature for evoke cost; sacrifice on ETB.
    Evoke,
    /// Cast for dash cost; gains haste, return to hand at end of turn.
    Dash,
    /// Cast from graveyard; exile other graveyard cards as additional cost.
    Escape,
    /// Cast from hand for madness cost when discarded.
    Madness,
    /// Cast for overload cost; replace "target" with "each".
    Overload,
    /// Cast for spectacle cost if opponent lost life this turn.
    Spectacle,
    /// Cast for emerge cost; sacrifice a creature, reduce cost.
    Emerge,
    /// Cast for blitz cost; gains haste, draw on death, sacrifice at EOT.
    Blitz,
    /// Cast a foretold card from exile for its foretell cost.
    Foretold,
}

impl AlternativeCost {
    /// Parse a keyword string to extract the alternative cost type and its mana cost.
    /// E.g. "Flashback:2 R" -> Some((Flashback, "2 R"))
    pub fn parse_keyword(keyword: &str) -> Option<(AlternativeCost, String)> {
        if let Some(cost) = keyword.strip_prefix("Flashback:") {
            Some((AlternativeCost::Flashback, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Evoke:") {
            Some((AlternativeCost::Evoke, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Dash:") {
            Some((AlternativeCost::Dash, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Madness:") {
            Some((AlternativeCost::Madness, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Overload:") {
            Some((AlternativeCost::Overload, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Spectacle:") {
            Some((AlternativeCost::Spectacle, cost.to_string()))
        } else if let Some(cost) = keyword.strip_prefix("Blitz:") {
            Some((AlternativeCost::Blitz, cost.to_string()))
        } else {
            None
        }
    }
}
