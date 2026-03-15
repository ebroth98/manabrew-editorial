use serde::{Deserialize, Serialize};

/// Optional additional costs for spells.
/// Mirrors Java's `OptionalCost.java` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionalCost {
    /// First kicker cost was paid.
    Kicker1,
    /// Second kicker cost was paid (for cards with two kicker costs).
    Kicker2,
    /// Buyback cost was paid (return spell to hand on resolution).
    Buyback,
    /// Entwine cost was paid (choose all modes).
    Entwine,
}
