//! Central parsing module for the Forge DSL.
//!
//! Mirrors Java's `FileSection.parseToMap()` + `CardTraitBase` accessor
//! methods. All pipe-delimited parameter parsing and typed access goes
//! through the [`Params`] wrapper — no code should use raw
//! `BTreeMap<String, String>` for DSL parameters.

pub mod compare;
pub mod keys;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    /// Get a parameter value or a default.
    /// Mirrors Java's `CardTraitBase.getParamOrDefault(String, String)`.
    pub fn get_or_default<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.0.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    /// Check if a parameter key exists.
    /// Mirrors Java's `CardTraitBase.hasParam(String)`.
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Set a parameter value.
    /// Mirrors Java's `CardTraitBase.putParam(String, String)`.
    pub fn put(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    /// Remove a parameter and return its value.
    /// Mirrors Java's `CardTraitBase.removeParam(String)`.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // ── Typed accessors ─────────────────────────────────────────────────

    /// Check if a boolean param is set to "True" (case-insensitive).
    /// Mirrors the common Java pattern `"True".equals(getParam(key))`.
    pub fn is_true(&self, key: &str) -> bool {
        self.0
            .get(key)
            .map_or(false, |v| v.eq_ignore_ascii_case("True"))
    }

    /// Parse a parameter as i32, returning None if absent or non-numeric.
    pub fn as_i32(&self, key: &str) -> Option<i32> {
        self.0.get(key).and_then(|v| v.trim().parse().ok())
    }

    /// Parse a parameter as usize, returning None if absent or non-numeric.
    pub fn as_usize(&self, key: &str) -> Option<usize> {
        self.0.get(key).and_then(|v| v.trim().parse().ok())
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
