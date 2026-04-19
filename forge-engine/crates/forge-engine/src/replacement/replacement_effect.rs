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

use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::card::Card;
use crate::card_trait_base::{CardTrait, CardTraitBase};
use crate::game::GameState;
use crate::game_loop::trigger_replacement_base::TriggerReplacementBase;
use crate::parsing::{keys, Params};
pub use crate::player::GameLossReason;

// Re-export so existing `use crate::replacement::replacement_effect::{ReplacementType, ReplacementLayer}`
// paths keep working.
pub use super::replacement_layer::ReplacementLayer;
pub use super::replacement_type::ReplacementType;

// ── ReplacementEffect ─────────────────────────────────────────────────────────

/// A parsed replacement effect from an `R$` line in a card script.
///
/// Params are stored exactly as they appear in the script so new param types
/// can be added without changing this struct.
///
/// Reference: Java `ReplacementEffect.java` in `forge/game/replacement/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementEffect {
    /// Shared trait base (host card, sVars, text-changes, map params).
    /// Mirrors Java `ReplacementEffect extends TriggerReplacementBase extends CardTraitBase`.
    /// Currently default-initialized by the parser; card factory population is
    /// a follow-up parity task so that `matches_valid_param` picks up
    /// `Invert*` entries from `map_params`.
    ///
    /// Boxed because `CardState` holds five inline `Option<ReplacementEffect>`
    /// fields (`loyalty_rep`, `defense_rep`, `saga_rep`, `adventure_rep`,
    /// `omen_rep`) and `TriggerReplacementBase → CardTraitBase` contains an
    /// `Option<CardState>`, which would otherwise form an infinite-sized
    /// type. `Trigger` does not need this because `CardState` only owns
    /// triggers via `Vec` (heap indirection already).
    #[serde(skip, default)]
    pub base: Box<TriggerReplacementBase>,
    /// The event type this effect intercepts.
    pub event: ReplacementType,
    /// The CR 616 layer this effect belongs to.
    pub layer: ReplacementLayer,
    /// Raw key→value pairs parsed from the pipe-separated script line.
    /// Keys do NOT include the trailing `$`.
    pub params: Params,
    /// Zones where this effect is active. Empty = active everywhere.
    /// Parsed from `ActiveZones$` parameter.
    /// TODO(java-parity): collapse into `base.valid_host_zones`.
    pub active_zones: Vec<ZoneType>,
    /// Temporary suppression flag used by effects like commander replacement.
    #[serde(default)]
    pub suppressed: bool,
}

impl CardTrait for ReplacementEffect {
    fn base(&self) -> &CardTraitBase {
        &self.base.card_trait_base
    }
}

impl ReplacementEffect {
    pub fn new(
        event: ReplacementType,
        layer: ReplacementLayer,
        params: Params,
        active_zones: Vec<ZoneType>,
    ) -> Self {
        let mut effect = Self {
            base: Box::new(TriggerReplacementBase::default()),
            event,
            layer,
            params,
            active_zones,
            suppressed: false,
        };
        effect.sync_trait_base_params();
        effect
    }

    fn sync_trait_base_params(&mut self) {
        let map = self
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.base.card_trait_base.set_map_params(map);
    }

    /// Returns `true` if this effect is active while the source card is in `zone`.
    ///
    /// An empty `active_zones` list means the effect is always active (mirrors
    /// Java `zonesCheck()` returning `true` when `activeZones` is empty).
    pub fn active_in_zone(&self, zone: ZoneType) -> bool {
        !self.suppressed && (self.active_zones.is_empty() || self.active_zones.contains(&zone))
    }

    /// Returns a human-readable description for this effect (from `Description$`).
    pub fn description(&self) -> &str {
        self.params
            .get(keys::DESCRIPTION)
            .unwrap_or("Replacement effect")
    }

    /// Returns `false` — effects don't track individual run state in our
    /// architecture. The handler's `has_run` set is cleared at the start of
    /// each `ReplacementHandler::run()` call, matching Java's reset semantics.
    pub fn has_run(&self) -> bool {
        false
    }

    /// Check requirements for this replacement effect against the current game state.
    ///
    /// - `PlayerTurn$` — verifies the active player matches the source card's controller.
    /// - `ActivePhases$` — verifies the current phase is in the comma-separated list.
    ///
    /// Returns `true` if all requirements are met (or if no requirements are specified).
    ///
    /// Mirrors Java `ReplacementEffect.requirementsCheck()`.
    pub fn requirements_check(&self, game: &GameState, source: &Card) -> bool {
        // PlayerTurn$ check — active player must be the source's controller
        if let Some(pt) = self.params.get(keys::PLAYER_TURN) {
            if pt == "True" && game.active_player() != source.controller {
                return false;
            }
        }

        // ActivePhases$ check — current phase must be in the listed phases
        if let Some(phases_str) = self.params.get(keys::ACTIVE_PHASES) {
            let current = game.turn.phase;
            let any_match = phases_str
                .split(',')
                .filter_map(|s| PhaseType::from_script_name(s.trim()))
                .any(|p| p == current);
            if !any_match {
                return false;
            }
        }

        true
    }

    /// Clone this replacement effect. Since `ReplacementEffect` derives `Clone`,
    /// this delegates to `self.clone()`.
    ///
    /// Mirrors Java `ReplacementEffect.copy()`.
    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// Return the `ReplaceWith$` param value if present — this is the SVar name
    /// of the replacement ability to execute.
    ///
    /// Mirrors Java `ReplacementEffect.ensureAbility()` which lazily resolves
    /// the SVar into a SpellAbility.
    pub fn ensure_ability(&self) -> Option<String> {
        self.params.get(keys::REPLACE_WITH).map(|s| s.to_string())
    }

    /// Check if this effect's event type matches the given event.
    ///
    /// For `AddCounter`, also matches `Moved` events when the effect handles
    /// counter-on-move (i.e. has a `CounterMap` interaction).
    ///
    /// Mirrors Java `ReplacementEffect.modeCheck()`.
    pub fn mode_check(&self, event: &ReplacementType) -> bool {
        if self.event == *event {
            return true;
        }
        // AddCounter effects can also intercept Moved events when they
        // involve a counter map (e.g. moving counters with the card).
        if self.event == ReplacementType::AddCounter && *event == ReplacementType::Moved {
            return self.params.get("CounterMap").is_some();
        }
        false
    }
}

// ── Helper filter functions ───────────────────────────────────────────────────
//
// `matches_valid_card` / `matches_valid_player` used to live here as free
// functions. They are now default methods on `CardTrait` (see
// `card_trait_base.rs`) so every subclass — `Trigger`, `ReplacementEffect`,
// and future `StaticAbility`/`SpellAbility` — gets the same API without
// per-module wrappers.

/// Check if a zone name string matches `zone`.
pub fn zone_matches(expr: &str, zone: ZoneType) -> bool {
    expr.split(',').any(|part| match part.trim() {
        "Battlefield" => zone == ZoneType::Battlefield,
        "Graveyard" => zone == ZoneType::Graveyard,
        "Hand" => zone == ZoneType::Hand,
        "Library" => zone == ZoneType::Library,
        "Exile" => zone == ZoneType::Exile,
        "Command" => zone == ZoneType::Command,
        "Stack" => zone == ZoneType::Stack,
        _ => false,
    })
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
        Some(s) => ReplacementType::smart_value_of(s),
        None => return None,
    };

    // Parse the layer (defaults to Other if not specified).
    let layer = params
        .get(keys::LAYER)
        .and_then(ReplacementLayer::smart_value_of)
        .unwrap_or(ReplacementLayer::Other);

    // Parse ActiveZones$ (comma- or space-separated list of zone names).
    let active_zones = params
        .get(keys::ACTIVE_ZONES)
        .map(|s| parse_zone_list(s))
        .unwrap_or_default();

    Some(ReplacementEffect::new(event, layer, params, active_zones))
}

/// Parse a comma- or space-separated zone list string into `ZoneType` values.
pub(super) fn parse_zone_list(s: &str) -> Vec<ZoneType> {
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
