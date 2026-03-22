//! Replacement effect parsing and types.
//!
//! Mirrors the Java Forge `forge/game/replacement/` package, specifically
//! `ReplacementEffect.java`.
//!
//! Card scripts encode replacement effects as `R$`-prefixed lines, e.g.:
//! ```text
//! R$ Event$ DamageDone | ActiveZones$ Battlefield | ValidCard$ Card.Self | Prevent$ True | Description$ Prevent all damage dealt to ~.
//! R$ Event$ Draw | ValidPlayer$ You | Description$ Skip your draw step.
//! R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | Description$ If ~ would die, exile it instead.
//! R$ Event$ Destroy | ValidCard$ Card.Self | Description$ ~ is indestructible.
//! ```

use serde::{Deserialize, Serialize};

use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::ids::PlayerId;
use crate::parsing::{keys, Params};

// Re-export so existing `use crate::replacement::replacement_effect::{ReplacementType, ReplacementLayer}`
// paths keep working.
pub use super::replacement_layer::ReplacementLayer;
pub use super::replacement_type::ReplacementType;

/// Reasons a player can lose the game.
/// Mirrors Java `GameLossReason` values used by `ValidLoseReason$`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameLossReason {
    LifeReachedZero,
    Poisoned,
    CommanderDamage,
    Milled,
    OpponentWon,
    SpellEffect,
}

// ── ReplacementEffect ─────────────────────────────────────────────────────────

/// A parsed replacement effect from an `R$` line in a card script.
///
/// Params are stored exactly as they appear in the script so new param types
/// can be added without changing this struct.
///
/// Reference: Java `ReplacementEffect.java` in `forge/game/replacement/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementEffect {
    /// The event type this effect intercepts.
    pub event: ReplacementType,
    /// The CR 616 layer this effect belongs to.
    pub layer: ReplacementLayer,
    /// Raw key→value pairs parsed from the pipe-separated script line.
    /// Keys do NOT include the trailing `$`.
    pub params: Params,
    /// Zones where this effect is active. Empty = active everywhere.
    /// Parsed from `ActiveZones$` parameter.
    pub active_zones: Vec<ZoneType>,
}

impl ReplacementEffect {
    /// Returns `true` if this effect is active while the source card is in `zone`.
    ///
    /// An empty `active_zones` list means the effect is always active (mirrors
    /// Java `zonesCheck()` returning `true` when `activeZones` is empty).
    pub fn active_in_zone(&self, zone: ZoneType) -> bool {
        self.active_zones.is_empty() || self.active_zones.contains(&zone)
    }

    /// Returns a human-readable description for this effect (from `Description$`).
    pub fn description(&self) -> &str {
        self.params
            .get(keys::DESCRIPTION)
            .unwrap_or("Replacement effect")
    }
}

// ── Helper filter functions ───────────────────────────────────────────────────

/// Check if a card matches a `ValidCard$` expression.
///
/// Supported tokens (mirrors Java `Card.isValid()` / `CardFilter`):
/// - `Card.Self`  — matches only the source card itself
/// - `Creature`   — matches creature permanents
/// - `Permanent`  — matches all permanents (no restriction)
/// - `Card`       — matches all cards (no restriction)
pub fn matches_valid_card(expr: &str, card: &Card, source: &Card) -> bool {
    valid_filter::matches_valid_card(expr, card, source)
}

/// Check if a player matches a `ValidPlayer$` expression.
///
/// - `You`              — the source card's controller
/// - `Opponent`         — not the source card's controller
/// - `Player.inGame`    — any player
/// - `Player` / empty   — any player (permissive default)
pub fn matches_valid_player(expr: &str, player: PlayerId, source: &Card) -> bool {
    valid_filter::matches_valid_player(expr, player, source.controller)
}

/// Check if a zone name string matches `zone`.
pub fn zone_matches(expr: &str, zone: ZoneType) -> bool {
    match expr.trim() {
        "Battlefield" => zone == ZoneType::Battlefield,
        "Graveyard" => zone == ZoneType::Graveyard,
        "Hand" => zone == ZoneType::Hand,
        "Library" => zone == ZoneType::Library,
        "Exile" => zone == ZoneType::Exile,
        "Command" => zone == ZoneType::Command,
        _ => false,
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse an `R$` (or `R:`) replacement-effect line from a card script.
///
/// Returns `None` if the line does not start with the `R$` / `R:` prefix or
/// has no recognisable `Event$` param.
///
/// # Format
///
/// ```text
/// R$ Event$ DamageDone | ActiveZones$ Battlefield | ValidCard$ Card.Self | Prevent$ True | Description$ Prevent all damage.
/// R$ Event$ Draw | ValidPlayer$ You | Description$ Skip your draw step.
/// R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self
/// ```
///
/// Reference: Java `ReplacementEffect.java` in `forge/game/replacement/`.
pub fn parse_replacement_effect(raw: &str) -> Option<ReplacementEffect> {
    let trimmed = raw.trim();
    // Accept "R$ ..." or "R: ..." prefixes (both appear in Forge card files).
    let body = if let Some(rest) = trimmed.strip_prefix("R$ ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("R:") {
        rest.trim_start()
    } else {
        return None;
    };

    // Parse "|"-separated "Key$ Value" pairs.
    let params = Params::from_raw(body);

    let event = match params.get(keys::EVENT) {
        Some(s) => ReplacementType::from_event_str(s),
        None => return None,
    };

    // Parse the layer (defaults to Other if not specified).
    let layer = params
        .get(keys::LAYER)
        .and_then(|s| ReplacementLayer::from_layer_str(s))
        .unwrap_or(ReplacementLayer::Other);

    // Parse ActiveZones$ (comma- or space-separated list of zone names).
    let active_zones = params
        .get(keys::ACTIVE_ZONES)
        .map(|s| parse_zone_list(s))
        .unwrap_or_default();

    Some(ReplacementEffect {
        event,
        layer,
        params,
        active_zones,
    })
}

/// Parse a comma- or space-separated zone list string into `ZoneType` values.
fn parse_zone_list(s: &str) -> Vec<ZoneType> {
    s.split(|c: char| c == ',' || c == ' ')
        .filter_map(|tok| match tok.trim() {
            "Battlefield" => Some(ZoneType::Battlefield),
            "Graveyard" => Some(ZoneType::Graveyard),
            "Hand" => Some(ZoneType::Hand),
            "Library" => Some(ZoneType::Library),
            "Exile" => Some(ZoneType::Exile),
            "Command" => Some(ZoneType::Command),
            _ => None,
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parser tests ──────────────────────────────────────────────────────

    #[test]
    fn parse_damage_prevention() {
        let raw = "R$ Event$ DamageDone | ActiveZones$ Battlefield | ValidCard$ Card.Self | Prevent$ True | Description$ Prevent all damage dealt to ~.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::DamageDone);
        assert_eq!(re.layer, ReplacementLayer::Other);
        assert_eq!(re.params.get(keys::PREVENT).unwrap(), "True");
        assert_eq!(re.active_zones, vec![ZoneType::Battlefield]);
    }

    #[test]
    fn parse_draw_skip() {
        let raw = "R$ Event$ Draw | ValidPlayer$ You | Description$ Skip your draw step.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Draw);
        assert_eq!(re.params.get(keys::VALID_PLAYER).unwrap(), "You");
        assert!(re.active_zones.is_empty());
    }

    #[test]
    fn parse_destroy_replacement() {
        let raw = "R$ Event$ Destroy | ValidCard$ Card.Self | Description$ ~ is indestructible.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Destroy);
        assert_eq!(re.params.get(keys::VALID_CARD).unwrap(), "Card.Self");
    }

    #[test]
    fn parse_moved_exile_instead() {
        let raw = "R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | NewDestination$ Exile | Description$ If ~ would die, exile it instead.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Moved);
        assert_eq!(re.params.get(keys::DESTINATION).unwrap(), "Graveyard");
        assert_eq!(re.params.get(keys::ORIGIN).unwrap(), "Battlefield");
        assert_eq!(re.params.get(keys::NEW_DESTINATION).unwrap(), "Exile");
    }

    #[test]
    fn parse_cant_happen_layer() {
        let raw = "R$ Event$ Destroy | Layer$ CantHappen | ValidCard$ Card.Self";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.layer, ReplacementLayer::CantHappen);
    }

    #[test]
    fn parse_r_colon_prefix() {
        let raw = "R: Event$ Draw | ValidPlayer$ You";
        let re = parse_replacement_effect(raw).expect("should parse R: prefix");
        assert_eq!(re.event, ReplacementType::Draw);
    }

    #[test]
    fn non_replacement_line_returns_none() {
        assert!(parse_replacement_effect("AB$ Mana | Cost$ T | Produced$ G").is_none());
        assert!(
            parse_replacement_effect("S$ Mode$ Continuous | Affected$ Creature.YouControl")
                .is_none()
        );
        assert!(parse_replacement_effect("").is_none());
    }

    // ── active_in_zone tests ──────────────────────────────────────────────

    #[test]
    fn active_in_zone_empty_means_always() {
        let raw = "R$ Event$ Draw | ValidPlayer$ You";
        let re = parse_replacement_effect(raw).unwrap();
        // Empty active_zones → active in all zones.
        assert!(re.active_in_zone(ZoneType::Battlefield));
        assert!(re.active_in_zone(ZoneType::Hand));
        assert!(re.active_in_zone(ZoneType::Graveyard));
    }

    #[test]
    fn active_in_zone_respects_active_zones() {
        let raw = "R$ Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True";
        let re = parse_replacement_effect(raw).unwrap();
        assert!(re.active_in_zone(ZoneType::Battlefield));
        assert!(!re.active_in_zone(ZoneType::Graveyard));
        assert!(!re.active_in_zone(ZoneType::Hand));
    }

    // ── New ReplacementType variant parsing tests ─────────────────────────

    #[test]
    fn parse_all_new_event_types() {
        for (event_str, expected) in [
            ("Tap", ReplacementType::Tap),
            ("Untap", ReplacementType::Untap),
            ("Mill", ReplacementType::Mill),
            ("Scry", ReplacementType::Scry),
            ("Explore", ReplacementType::Explore),
            ("Cascade", ReplacementType::Cascade),
            ("Learn", ReplacementType::Learn),
            ("Proliferate", ReplacementType::Proliferate),
            ("Transform", ReplacementType::Transform),
            ("TurnFaceUp", ReplacementType::TurnFaceUp),
            ("RollDice", ReplacementType::RollDice),
        ] {
            let raw = format!("R$ Event$ {event_str} | Description$ test");
            let re = parse_replacement_effect(&raw).expect(&format!("should parse {event_str}"));
            assert_eq!(re.event, expected, "failed for {event_str}");
        }
    }
}
