//! Target choices for spell abilities.
//!
//! Mirrors Java's `spellability/TargetChoices.java` — a container holding
//! the actual selected targets for a spell ability.

use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// Targets chosen for a single ability in the SubAbility chain.
/// Mirrors Java's `TargetChoices` which holds selected targets (cards, players, stack entries).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetChoices {
    pub target_player: Option<PlayerId>,
    pub target_card: Option<CardId>,
    /// ID of a targeted stack entry (for Counter effects).
    pub target_stack_entry: Option<u32>,
}
