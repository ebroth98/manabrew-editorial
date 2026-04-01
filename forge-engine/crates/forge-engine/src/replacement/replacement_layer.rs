//! Replacement effect layer ordering.
//!
//! Mirrors Java `ReplacementLayer.java` in `forge/game/replacement/`.

use serde::{Deserialize, Serialize};

/// CR 614 / CR 616 layer ordering for replacement effects.
///
/// Multiple replacement effects that apply to the same event are applied in
/// the order below. Within the same layer the affected player chooses the order.
///
/// Reference: CR 616.1, Java `ReplacementLayer.java`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReplacementLayer {
    /// CR 614.17 — effects that say an event "can't happen" (highest priority).
    CantHappen = 0,
    /// CR 616.1b — control-changing replacement effects.
    Control = 1,
    /// CR 616.1c — copy replacement effects.
    Copy = 2,
    /// CR 616.1d — transform replacement effects.
    Transform = 3,
    /// All other replacement effects (damage prevention, zone rerouting, etc.).
    Other = 4,
}

impl ReplacementLayer {
    /// Alias for `from_layer_str`. Mirrors Java `ReplacementLayer.smartValueOf()`.
    pub fn smart_value_of(value: &str) -> Option<Self> {
        Self::from_layer_str(value)
    }

    /// Parse a `Layer$` value string. Returns `None` if unrecognised.
    pub fn from_layer_str(s: &str) -> Option<Self> {
        match s.trim() {
            "CantHappen" => Some(Self::CantHappen),
            "Control" => Some(Self::Control),
            "Copy" => Some(Self::Copy),
            "Transform" => Some(Self::Transform),
            "Other" => Some(Self::Other),
            _ => None,
        }
    }
}
