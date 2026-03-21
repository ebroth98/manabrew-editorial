//! Keyword interface trait definition.
//!
//! Ported from Java's `KeywordInterface.java` in `forge/game/keyword/`.

use super::keyword_instance::Keyword;

/// Trait defining the contract for keyword instances.
/// Mirrors Java's `KeywordInterface`.
pub trait KeywordInterface: Clone {
    /// Get the original keyword string (e.g. "Flying", "Kicker:1 R").
    fn get_original(&self) -> &str;

    /// Get the keyword enum variant.
    fn get_keyword(&self) -> Keyword;

    /// Get the display title for this keyword.
    fn get_title(&self) -> String;

    /// Get the reminder text for this keyword.
    fn get_reminder_text(&self) -> String;

    /// Get the numeric amount (default 1 for most keywords).
    fn get_amount(&self) -> i32 {
        1
    }

    /// Get the amount as a string.
    fn get_amount_string(&self) -> String {
        self.get_amount().to_string()
    }

    /// Whether this keyword is intrinsic to the card (printed on it).
    fn is_intrinsic(&self) -> bool;

    /// Set whether this keyword is intrinsic.
    fn set_intrinsic(&mut self, value: bool);

    /// Get the unique index for this keyword instance.
    fn get_idx(&self) -> i64;

    /// Set the unique index.
    fn set_idx(&mut self, i: i64);

    /// Whether this keyword instance is redundant given existing keywords.
    /// Takes keyword strings for compatibility — mirrors Java's `redundant(Collection)`.
    fn redundant(&self, _existing_keywords: &[String]) -> bool {
        false
    }
}
