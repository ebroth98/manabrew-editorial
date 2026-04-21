//! Central parsing module for the Forge DSL.
//!
//! Mirrors Java's `FileSection.parseToMap()` + `CardTraitBase` accessor
//! methods. All pipe-delimited parameter parsing and typed access goes
//! through the [`Params`] wrapper — no code should use raw
//! `BTreeMap<String, String>` for DSL parameters.

pub mod compare;
pub mod keys;

use std::collections::BTreeMap;

use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

pub fn raw_has_key(raw: &str, key: &str) -> bool {
    raw_get(raw, key).is_some()
}

pub fn raw_has_any(raw: &str, keys: &[&str]) -> bool {
    raw.split('|').any(|part| {
        let part = part.trim();
        let Some(idx) = part.find('$') else {
            return false;
        };
        let part_key = part[..idx].trim();
        keys.iter().any(|key| part_key == *key)
    })
}

pub fn raw_get<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    raw.split('|').find_map(|part| {
        let part = part.trim();
        let Some(idx) = part.find('$') else {
            return None;
        };
        if part[..idx].trim() != key {
            return None;
        }
        let value = if part[idx..].starts_with("$ ") {
            &part[idx + 2..]
        } else {
            &part[idx + 1..]
        };
        Some(value.trim())
    })
}

// ── Params wrapper ──────────────────────────────────────────────────────────

/// Typed wrapper around parsed DSL parameters.
///
/// Replaces raw `BTreeMap<String, String>` everywhere. Mirrors Java's
/// `CardTraitBase.mapParams` with its `getParam`/`hasParam`/`matchesValidParam`
/// accessor methods.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Params(BTreeMap<String, String>);

impl Params {
    /// Parse a pipe-delimited DSL string into parameters.
    ///
    /// Handles both `Key$ Value` and `Key$Value` (no trailing space) formats.
    /// Mirrors Java's `FileSection.parseToMap()`.
    pub fn from_raw(raw: &str) -> Self {
        crate::perf::increment_params_parse();
        let mut map = BTreeMap::new();
        for part in raw.split('|') {
            let part = part.trim();
            if let Some(idx) = part.find("$ ") {
                let key = part[..idx].trim().to_string();
                let value = part[idx + 2..].trim().to_string();
                map.insert(key, value);
            } else if let Some(idx) = part.find('$') {
                let key = part[..idx].trim().to_string();
                let value = part[idx + 1..].trim().to_string();
                map.insert(key, value);
            }
        }
        Params(map)
    }

    /// Create from an existing map (for migration from raw BTreeMap).
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        Params(map)
    }

    /// Access the underlying map (escape hatch for incremental migration).
    pub fn inner(&self) -> &BTreeMap<String, String> {
        &self.0
    }

    /// Consume and return the underlying map.
    pub fn into_inner(self) -> BTreeMap<String, String> {
        self.0
    }

    // ── Core accessors (mirror Java CardTraitBase) ──────────────────────

    /// Get a parameter value by key.
    /// Mirrors Java's `CardTraitBase.getParam(String)`.
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&str> {
        crate::perf::increment_params_lookup();
        self.0.get(key.as_ref()).map(|s| s.as_str())
    }

    /// Get a parameter value or a default.
    /// Mirrors Java's `CardTraitBase.getParamOrDefault(String, String)`.
    pub fn get_or_default<'a, K: AsRef<str>>(&'a self, key: K, default: &'a str) -> &'a str {
        crate::perf::increment_params_lookup();
        self.0
            .get(key.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    /// Check if a parameter key exists.
    /// Mirrors Java's `CardTraitBase.hasParam(String)`.
    pub fn has<K: AsRef<str>>(&self, key: K) -> bool {
        crate::perf::increment_params_lookup();
        self.0.contains_key(key.as_ref())
    }

    /// Set a parameter value.
    /// Mirrors Java's `CardTraitBase.putParam(String, String)`.
    pub fn put(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    /// Remove a parameter and return its value.
    /// Mirrors Java's `CardTraitBase.removeParam(String)`.
    pub fn remove<K: AsRef<str>>(&mut self, key: K) -> Option<String> {
        self.0.remove(key.as_ref())
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check whether any key exists without recording accessor instrumentation.
    ///
    /// Intended for coarse hot-path gates before a caller decides whether to
    /// perform many instrumented typed lookups.
    pub fn contains_any_key(&self, keys: &[&str]) -> bool {
        keys.iter().any(|key| self.0.contains_key(*key))
    }

    // ── Typed accessors ─────────────────────────────────────────────────

    /// Check if a boolean param is set to "True" (case-insensitive).
    /// Mirrors the common Java pattern `"True".equals(getParam(key))`.
    pub fn is_true<K: AsRef<str>>(&self, key: K) -> bool {
        crate::perf::increment_params_lookup();
        self.0
            .get(key.as_ref())
            .map_or(false, |v| v.eq_ignore_ascii_case("True"))
    }

    /// Parse a parameter as i32, returning None if absent or non-numeric.
    pub fn as_i32<K: AsRef<str>>(&self, key: K) -> Option<i32> {
        crate::perf::increment_params_lookup();
        self.0.get(key.as_ref()).and_then(|v| v.trim().parse().ok())
    }

    /// Parse a parameter as usize, returning None if absent or non-numeric.
    pub fn as_usize<K: AsRef<str>>(&self, key: K) -> Option<usize> {
        crate::perf::increment_params_lookup();
        self.0.get(key.as_ref()).and_then(|v| v.trim().parse().ok())
    }

    /// Get a parameter value, cloning it into an owned String.
    pub fn get_cloned(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    // ── Diagnostics ────────────────────────────────────────────────────

    /// Get a required parameter, logging a warning if missing.
    ///
    /// Use this instead of `.get()` when the parameter is expected to exist.
    /// Missing parameters are logged with context for debugging card scripts.
    pub fn require(&self, key: &str, context: &str) -> Option<&str> {
        match self.get(key) {
            Some(v) => Some(v),
            None => {
                eprintln!("[parse] missing required param '{}' in {}", key, context);
                None
            }
        }
    }

    /// Get a required parameter as an owned String, logging if missing.
    pub fn require_cloned(&self, key: &str, context: &str) -> Option<String> {
        self.require(key, context).map(|s| s.to_string())
    }

    // ── Iteration ───────────────────────────────────────────────────────

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Adapter: convert a parse result to Option, logging failures only when
/// the raw input looks like it was *intended* to be the given kind.
///
/// Card scripts pass all ability lines to all parsers — most `None` results
/// are intentional (e.g., an `AB$` line passed to `parse_static_ability`).
/// This function only warns when the line's prefix matches the expected kind.
///
/// ```ignore
/// .filter_map(|raw| parse_or_warn(parse_static_ability(raw), "StaticAbility", raw))
/// ```
pub fn parse_or_warn<T>(result: Option<T>, kind: &str, raw: &str) -> Option<T> {
    if result.is_none() {
        let trimmed = raw.trim();
        let should_warn = match kind {
            "StaticAbility" => trimmed.starts_with("S$") || trimmed.starts_with("S:"),
            "ReplacementEffect" => trimmed.starts_with("R$") || trimmed.starts_with("R:"),
            "Trigger" => trimmed.starts_with("T$") || trimmed.starts_with("T:"),
            "ActivatedAbility" => {
                // Only AB$ lines are activated abilities. SP$ (spell) and DB$
                // (sub-ability) lines are resolved via build_spell_ability, not
                // parse_activated_ability — their None result is intentional.
                trimmed.starts_with("AB$") || trimmed.starts_with("AB:")
            }
            _ => false,
        };
        if should_warn {
            let preview: String = trimmed.chars().take(100).collect();
            eprintln!("[parse] failed to parse {} from: {}", kind, preview);
        }
    }
    result
}

// ── Conversions for incremental migration ───────────────────────────────────

impl From<BTreeMap<String, String>> for Params {
    fn from(map: BTreeMap<String, String>) -> Self {
        Params(map)
    }
}

impl From<Params> for BTreeMap<String, String> {
    fn from(params: Params) -> Self {
        params.0
    }
}

// ── Shared DSL parsing utilities ─────────────────────────────────────────────

/// Strip a `/Times.N` multiplier suffix from a filter string.
/// Returns (filter_without_suffix, multiplier).
///
/// Used by SVar `Count$Valid` expressions and cost parsing.
/// Example: `"Enchantment.Other/Times.2"` → `("Enchantment.Other", 2)`
pub fn strip_times_multiplier(s: &str) -> (&str, i32) {
    if let Some(idx) = s.find("/Times.") {
        let mult_str = &s[idx + 7..];
        let mult = mult_str.parse::<i32>().unwrap_or(1);
        (&s[..idx], mult)
    } else {
        (s, 1)
    }
}

/// Map an "Enchant <type>" keyword value to a ValidTgts$ filter string.
/// Used by aura targeting (ability_factory) and aura SBA legality (action.rs).
///
/// Example: `"creature"` → `"Creature"`, `"land"` → `"Land"`
fn normalize_enchant_type(enchant_type: &str) -> &str {
    enchant_type
        .split_once(':')
        .map(|(kind, _)| kind)
        .unwrap_or(enchant_type)
        .trim()
}

pub fn enchant_type_to_valid_tgts(enchant_type: &str) -> &'static str {
    match normalize_enchant_type(enchant_type).to_lowercase().as_str() {
        "creature" => "Creature",
        "land" => "Land",
        "artifact" => "Artifact",
        "enchantment" => "Enchantment",
        "planeswalker" => "Planeswalker",
        "permanent" => "Permanent",
        "player" => "Player",
        "creature or player" => "Creature,Player",
        _ => "Permanent",
    }
}

/// Build a minimal targeting params string from an Enchant keyword payload.
/// Handles special cases like `Creature.inZoneGraveyard` used by Animate Dead.
pub fn enchant_type_to_target_params(enchant_type: &str) -> String {
    let normalized = normalize_enchant_type(enchant_type);
    let lower = normalized.to_lowercase();
    if lower == "creature.inzonegraveyard" {
        return "Origin$ Graveyard | ValidTgts$ Creature".to_string();
    }
    format!("ValidTgts$ {}", enchant_type_to_valid_tgts(normalized))
}

/// Check if a card type matches an "Enchant <type>" keyword value.
/// Used by aura SBA to verify the enchant restriction is still met.
///
/// Example: `enchant_type_matches_card("creature", card)` → true if card is a creature
pub fn enchant_type_matches_card(enchant_type: &str, card: &crate::card::CardInstance) -> bool {
    match normalize_enchant_type(enchant_type).to_lowercase().as_str() {
        "creature" => card.zone == ZoneType::Battlefield && card.is_creature(),
        "creature.inzonegraveyard" => card.zone == ZoneType::Graveyard && card.is_creature(),
        "land" => card.zone == ZoneType::Battlefield && card.is_land(),
        "artifact" => card.zone == ZoneType::Battlefield && card.type_line.is_artifact(),
        "enchantment" => card.zone == ZoneType::Battlefield && card.type_line.is_enchantment(),
        "planeswalker" => card.zone == ZoneType::Battlefield && card.type_line.is_planeswalker(),
        "permanent" | "" => card.zone == ZoneType::Battlefield,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_pipe_params() {
        let params = Params::from_raw("Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield");
        assert_eq!(params.get("Mode"), Some("ChangesZone"));
        assert_eq!(params.get("Origin"), Some("Any"));
        assert_eq!(params.get("Destination"), Some("Battlefield"));
    }

    #[test]
    fn parse_bare_dollar_no_space() {
        // This was a bug in the duplicated parsers — they only handled "$ "
        let params = Params::from_raw("Key$Value | Other$ Spaced");
        assert_eq!(params.get("Key"), Some("Value"));
        assert_eq!(params.get("Other"), Some("Spaced"));
    }

    #[test]
    fn is_true_case_insensitive() {
        let params = Params::from_raw("Hidden$ True | Mandatory$ true | Other$ false");
        assert!(params.is_true("Hidden"));
        assert!(params.is_true("Mandatory"));
        assert!(!params.is_true("Other"));
        assert!(!params.is_true("Missing"));
    }

    #[test]
    fn numeric_accessors() {
        let params = Params::from_raw("Amount$ 3 | Bad$ notanumber");
        assert_eq!(params.as_i32("Amount"), Some(3));
        assert_eq!(params.as_usize("Amount"), Some(3));
        assert_eq!(params.as_i32("Bad"), None);
        assert_eq!(params.as_i32("Missing"), None);
    }

    #[test]
    fn get_or_default_works() {
        let params = Params::from_raw("Mode$ Continuous");
        assert_eq!(params.get_or_default("Mode", "None"), "Continuous");
        assert_eq!(params.get_or_default("Missing", "fallback"), "fallback");
    }

    #[test]
    fn serde_roundtrip() {
        let params = Params::from_raw("Mode$ Test | Amount$ 5");
        let json = serde_json::to_string(&params).unwrap();
        let deserialized: Params = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.get("Mode"), Some("Test"));
        assert_eq!(deserialized.get("Amount"), Some("5"));
    }
}
