//! Replacement effect parsing and types.
//!
//! Mirrors the Java Forge `forge/game/replacement/` package, specifically
//! `ReplacementEffect.java`, `ReplacementType.java`, `ReplacementLayer.java`,
//! and `ReplacementResult.java`.
//!
//! Card scripts encode replacement effects as `R$`-prefixed lines, e.g.:
//! ```text
//! R$ Event$ DamageDone | ActiveZones$ Battlefield | ValidCard$ Card.Self | Prevent$ True | Description$ Prevent all damage dealt to ~.
//! R$ Event$ Draw | ValidPlayer$ You | Description$ Skip your draw step.
//! R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | Description$ If ~ would die, exile it instead.
//! R$ Event$ Destroy | ValidCard$ Card.Self | Description$ ~ is indestructible.
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use forge_foundation::ZoneType;

use crate::card::{valid_filter, CardInstance};
use crate::ids::PlayerId;

// ── ReplacementType ───────────────────────────────────────────────────────────

/// The type of game event a replacement effect intercepts.
///
/// Mirrors Java `ReplacementType` enum. Each variant corresponds to an
/// `Event$ <Value>` entry in the card script.
///
/// Reference: Java `ReplacementType.java` in `forge/game/replacement/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplacementType {
    /// `Event$ DamageDone` — damage being dealt to a card or player.
    DamageDone,

    /// `Event$ Draw` — a single card draw.
    Draw,

    /// `Event$ DrawCards` — multiple card draws at once.
    DrawCards,

    /// `Event$ Destroy` — a permanent being destroyed.
    Destroy,

    /// `Event$ Moved` — a card moving between zones (ETB, dies, exile, etc.).
    Moved,

    /// `Event$ GainLife` — a player gaining life.
    GainLife,

    /// `Event$ AddCounter` — a counter being added to a permanent or player.
    AddCounter,

    /// `Event$ GameLoss` — a player losing the game (e.g. Platinum Angel).
    GameLoss,

    /// `Event$ GameWin` — a player winning the game.
    GameWin,

    /// `Event$ CreateToken` — creating token(s).
    CreateToken,

    /// `Event$ Counter` — countering a spell.
    Counter,

    /// `Event$ ProduceMana` — mana being produced (for doublers like Mirari's Wake).
    ProduceMana,

    /// Any event type not yet recognised — stored but not applied.
    Other(String),
}

// ── ReplacementLayer ──────────────────────────────────────────────────────────

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

// ── ReplacementResult ─────────────────────────────────────────────────────────

/// The result of attempting to apply a replacement effect.
///
/// Mirrors Java `ReplacementResult` enum.
///
/// Reference: Java `ReplacementResult.java` in `forge/game/replacement/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplacementResult {
    /// The event was fully replaced; no further processing needed.
    Replaced,
    /// This effect did not apply; continue checking other effects.
    NotReplaced,
    /// The event was prevented (damage prevention, etc.).
    Prevented,
    /// The event parameters were modified; re-run replacement check from start.
    Updated,
    /// The event was skipped entirely (e.g. "skip your draw step").
    Skipped,
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
    pub params: BTreeMap<String, String>,
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

    /// Returns `true` if this effect can replace a `Destroy` event targeting `target`.
    ///
    /// Used to implement indestructible permanents.
    ///
    /// Mirrors Java `ReplaceDestroy.canReplace()`.
    pub fn can_replace_destroy(&self, target: &CardInstance, source: &CardInstance) -> bool {
        if self.event != ReplacementType::Destroy {
            return false;
        }
        // ValidCard$ check — only match if target satisfies the filter.
        if let Some(valid) = self.params.get("ValidCard") {
            if !matches_valid_card(valid, target, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `Draw` event for `player_id`.
    ///
    /// Mirrors Java `ReplaceDraw.canReplace()`.
    pub fn can_replace_draw(&self, player_id: PlayerId, source: &CardInstance) -> bool {
        if self.event != ReplacementType::Draw {
            return false;
        }
        // ValidPlayer$ check.
        if let Some(valid) = self.params.get("ValidPlayer") {
            if !matches_valid_player(valid, player_id, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `DamageDone` event.
    ///
    /// `target_is_player` distinguishes damage targeting a player vs. a card.
    ///
    /// Mirrors Java `ReplaceDamage.canReplace()`.
    pub fn can_replace_damage(&self, target_is_player: bool, _source: &CardInstance) -> bool {
        if self.event != ReplacementType::DamageDone {
            return false;
        }
        // ValidTarget$ check (simplified — broad token matching).
        if let Some(valid_target) = self.params.get("ValidTarget") {
            let target_matches = match valid_target.trim() {
                "Player" => target_is_player,
                "Card" | "Creature" | "Permanent" => !target_is_player,
                "Any" | "CardOrPlayer" => true,
                _ => false,
            };
            if !target_matches {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `GainLife` event for `player_id`.
    pub fn can_replace_gain_life(&self, player_id: PlayerId, source: &CardInstance) -> bool {
        if self.event != ReplacementType::GainLife {
            return false;
        }
        if let Some(valid) = self.params.get("ValidPlayer") {
            if !matches_valid_player(valid, player_id, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `CreateToken` event for `player_id`.
    pub fn can_replace_create_token(&self, player_id: PlayerId, source: &CardInstance) -> bool {
        if self.event != ReplacementType::CreateToken {
            return false;
        }
        if let Some(valid) = self.params.get("ValidPlayer") {
            if !matches_valid_player(valid, player_id, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace an `AddCounter` event.
    pub fn can_replace_add_counter(&self, target: &CardInstance, source: &CardInstance) -> bool {
        if self.event != ReplacementType::AddCounter {
            return false;
        }
        if let Some(valid) = self.params.get("ValidCard") {
            if !matches_valid_card(valid, target, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `GameLoss` event for `player_id`.
    pub fn can_replace_game_loss(&self, player_id: PlayerId, source: &CardInstance) -> bool {
        if self.event != ReplacementType::GameLoss {
            return false;
        }
        if let Some(valid) = self.params.get("ValidPlayer") {
            if !matches_valid_player(valid, player_id, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `GameWin` event for `player_id`.
    pub fn can_replace_game_win(&self, player_id: PlayerId, source: &CardInstance) -> bool {
        if self.event != ReplacementType::GameWin {
            return false;
        }
        if let Some(valid) = self.params.get("ValidPlayer") {
            if !matches_valid_player(valid, player_id, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `Counter` (counterspell) event.
    pub fn can_replace_counter(&self, target: &CardInstance, source: &CardInstance) -> bool {
        if self.event != ReplacementType::Counter {
            return false;
        }
        if let Some(valid) = self.params.get("ValidCard") {
            if !matches_valid_card(valid, target, source) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `ProduceMana` event (mana doublers).
    pub fn can_replace_produce_mana(
        &self,
        source: &CardInstance,
        activator: PlayerId,
        host: &CardInstance,
    ) -> bool {
        if self.event != ReplacementType::ProduceMana {
            return false;
        }
        // ValidCard$ — which permanent is producing mana
        if let Some(valid) = self.params.get("ValidCard") {
            if valid != "Permanent" && valid != "Card" {
                if !matches_valid_card(valid, source, host) {
                    return false;
                }
            }
        }
        // ValidActivator$ / ValidPlayer$ — who controls the producing permanent
        if let Some(valid_player) = self
            .params
            .get("ValidActivator")
            .or(self.params.get("ValidPlayer"))
        {
            if !matches_valid_player(valid_player, activator, host) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if this effect can replace a `Moved` (zone change) event.
    ///
    /// Checks `Destination$`, `Origin$`, and `ValidCard$` parameters.
    ///
    /// Mirrors Java `ReplaceMoved.canReplace()`.
    pub fn can_replace_moved(
        &self,
        origin: ZoneType,
        destination: ZoneType,
        card: &CardInstance,
        source: &CardInstance,
    ) -> bool {
        if self.event != ReplacementType::Moved {
            return false;
        }
        // Destination$ check.
        if let Some(dest) = self.params.get("Destination") {
            if !zone_matches(dest, destination) {
                return false;
            }
        }
        // Origin$ check.
        if let Some(orig) = self.params.get("Origin") {
            if !zone_matches(orig, origin) {
                return false;
            }
        }
        // ValidCard$ check.
        if let Some(valid) = self.params.get("ValidCard") {
            if !matches_valid_card(valid, card, source) {
                return false;
            }
        }
        true
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
pub fn matches_valid_card(expr: &str, card: &CardInstance, source: &CardInstance) -> bool {
    valid_filter::matches_valid_card(expr, card, source)
}

/// Check if a player matches a `ValidPlayer$` expression.
///
/// - `You`              — the source card's controller
/// - `Opponent`         — not the source card's controller
/// - `Player.inGame`    — any player
/// - `Player` / empty   — any player (permissive default)
pub fn matches_valid_player(expr: &str, player: PlayerId, source: &CardInstance) -> bool {
    valid_filter::matches_valid_player(expr, player, source.controller)
}

/// Check if a zone name string matches `zone`.
fn zone_matches(expr: &str, zone: ZoneType) -> bool {
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
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    for segment in body.split('|') {
        let seg = segment.trim();
        if let Some(idx) = seg.find("$ ") {
            let key = seg[..idx].trim().to_string();
            let val = seg[idx + 2..].trim().to_string();
            params.insert(key, val);
        }
    }

    let event = match params.get("Event").map(String::as_str) {
        Some("DamageDone") => ReplacementType::DamageDone,
        Some("Draw") => ReplacementType::Draw,
        Some("DrawCards") => ReplacementType::DrawCards,
        Some("Destroy") => ReplacementType::Destroy,
        Some("Moved") => ReplacementType::Moved,
        Some("GainLife") => ReplacementType::GainLife,
        Some("AddCounter") => ReplacementType::AddCounter,
        Some("GameLoss") => ReplacementType::GameLoss,
        Some("GameWin") => ReplacementType::GameWin,
        Some("CreateToken") => ReplacementType::CreateToken,
        Some("Counter") => ReplacementType::Counter,
        Some("ProduceMana") => ReplacementType::ProduceMana,
        Some(other) => ReplacementType::Other(other.to_string()),
        None => return None,
    };

    // Parse the layer (defaults to Other if not specified).
    let layer = match params.get("Layer").map(String::as_str) {
        Some("CantHappen") => ReplacementLayer::CantHappen,
        Some("Control") => ReplacementLayer::Control,
        Some("Copy") => ReplacementLayer::Copy,
        Some("Transform") => ReplacementLayer::Transform,
        _ => ReplacementLayer::Other,
    };

    // Parse ActiveZones$ (comma- or space-separated list of zone names).
    let active_zones = params
        .get("ActiveZones")
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
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    use crate::card::CardInstance;
    use crate::ids::{CardId, PlayerId};

    fn make_creature(id: u32, owner: u32) -> CardInstance {
        CardInstance::new(
            CardId(id),
            "Test".to_string(),
            PlayerId(owner),
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        )
    }

    fn make_land(id: u32, owner: u32) -> CardInstance {
        CardInstance::new(
            CardId(id),
            "Forest".to_string(),
            PlayerId(owner),
            CardTypeLine::parse("Basic Land - Forest"),
            ManaCost::parse(""),
            ColorSet::GREEN,
            None,
            None,
            vec![],
            vec![],
        )
    }

    // ── Parser tests ──────────────────────────────────────────────────────

    #[test]
    fn parse_damage_prevention() {
        let raw = "R$ Event$ DamageDone | ActiveZones$ Battlefield | ValidCard$ Card.Self | Prevent$ True | Description$ Prevent all damage dealt to ~.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::DamageDone);
        assert_eq!(re.layer, ReplacementLayer::Other);
        assert_eq!(re.params["Prevent"], "True");
        assert_eq!(re.active_zones, vec![ZoneType::Battlefield]);
    }

    #[test]
    fn parse_draw_skip() {
        let raw = "R$ Event$ Draw | ValidPlayer$ You | Description$ Skip your draw step.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Draw);
        assert_eq!(re.params["ValidPlayer"], "You");
        assert!(re.active_zones.is_empty());
    }

    #[test]
    fn parse_destroy_replacement() {
        let raw = "R$ Event$ Destroy | ValidCard$ Card.Self | Description$ ~ is indestructible.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Destroy);
        assert_eq!(re.params["ValidCard"], "Card.Self");
    }

    #[test]
    fn parse_moved_exile_instead() {
        let raw = "R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | NewDestination$ Exile | Description$ If ~ would die, exile it instead.";
        let re = parse_replacement_effect(raw).expect("should parse");
        assert_eq!(re.event, ReplacementType::Moved);
        assert_eq!(re.params["Destination"], "Graveyard");
        assert_eq!(re.params["Origin"], "Battlefield");
        assert_eq!(re.params["NewDestination"], "Exile");
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

    // ── can_replace_* tests ───────────────────────────────────────────────

    #[test]
    fn can_replace_destroy_self() {
        let source = make_creature(0, 0);
        let re = parse_replacement_effect("R$ Event$ Destroy | ValidCard$ Card.Self").unwrap();
        // Matches when target == source.
        assert!(re.can_replace_destroy(&source, &source));
    }

    #[test]
    fn can_replace_destroy_other_card_excluded() {
        let source = make_creature(0, 0);
        let other = make_creature(1, 0);
        let re = parse_replacement_effect("R$ Event$ Destroy | ValidCard$ Card.Self").unwrap();
        // Does NOT match when target is a different card.
        assert!(!re.can_replace_destroy(&other, &source));
    }

    #[test]
    fn can_replace_draw_matches_controller() {
        let source = make_creature(0, 0); // controller = PlayerId(0)
        let re = parse_replacement_effect("R$ Event$ Draw | ValidPlayer$ You").unwrap();
        assert!(re.can_replace_draw(PlayerId(0), &source));
        assert!(!re.can_replace_draw(PlayerId(1), &source));
    }

    #[test]
    fn can_replace_draw_opponent() {
        let source = make_creature(0, 0); // controller = PlayerId(0)
        let re = parse_replacement_effect("R$ Event$ Draw | ValidPlayer$ Opponent").unwrap();
        assert!(!re.can_replace_draw(PlayerId(0), &source));
        assert!(re.can_replace_draw(PlayerId(1), &source));
    }

    #[test]
    fn can_replace_damage_player_target() {
        let source = make_creature(0, 0);
        let re =
            parse_replacement_effect("R$ Event$ DamageDone | ValidTarget$ Player | Prevent$ True")
                .unwrap();
        assert!(re.can_replace_damage(true, &source)); // targets player
        assert!(!re.can_replace_damage(false, &source)); // targets card → no match
    }

    #[test]
    fn can_replace_moved_graveyard_from_battlefield() {
        let source = make_creature(0, 0);
        let re = parse_replacement_effect(
            "R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self",
        )
        .unwrap();
        // Correct origin/destination + self → matches.
        assert!(re.can_replace_moved(ZoneType::Battlefield, ZoneType::Graveyard, &source, &source));
    }

    #[test]
    fn can_replace_moved_wrong_destination() {
        let source = make_creature(0, 0);
        let re = parse_replacement_effect(
            "R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self",
        )
        .unwrap();
        // Destination is Exile, not Graveyard → no match.
        assert!(!re.can_replace_moved(ZoneType::Battlefield, ZoneType::Exile, &source, &source));
    }

    #[test]
    fn can_replace_moved_wrong_card() {
        let source = make_creature(0, 0);
        let other = make_creature(1, 0);
        let re = parse_replacement_effect(
            "R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self",
        )
        .unwrap();
        // `other` is not `source` → no match with Card.Self.
        assert!(!re.can_replace_moved(ZoneType::Battlefield, ZoneType::Graveyard, &other, &source));
    }

    #[test]
    fn land_matches_card_filter() {
        let source = make_creature(0, 0);
        let land = make_land(1, 0);
        // "Permanent" imposes no restriction — land passes.
        assert!(matches_valid_card("Permanent", &land, &source));
        // "Creature" restricts to creatures — land fails.
        assert!(!matches_valid_card("Creature", &land, &source));
    }
}
