//! Re-export of the StaticMode enum for Java parity naming.
//! The actual enum is defined in `mod.rs`.
pub use crate::staticability::StaticMode;

/// Parse a mode string into a StaticMode, tolerating common aliases.
/// Mirrors Java's `StaticAbilityMode.smartValueOf()`.
///
/// Delegates to the mode-parsing logic in `parse_static_ability` for consistency,
/// avoiding drift between two separate match tables.
pub fn smart_value_of(s: &str) -> StaticMode {
    // Use the canonical mode parser to avoid drift between two match tables.
    // Build a minimal S$ line and parse it.
    let synthetic = format!("S$ Mode$ {}", s.trim());
    match crate::staticability::parse_static_ability(&synthetic) {
        Some(sa) => sa.mode,
        None => StaticMode::Other(s.trim().to_string()),
    }
}
