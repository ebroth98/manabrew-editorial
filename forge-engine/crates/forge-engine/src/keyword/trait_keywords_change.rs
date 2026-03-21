//! Interface for keyword changes.
//!
//! Ported from Java's `IKeywordsChange.java` in `forge/game/keyword/`.

use super::keyword_collection::KeywordCollection;

/// Trait for objects that can apply keyword changes to a collection.
/// Mirrors Java's `IKeywordsChange` interface.
pub trait KeywordsChange {
    /// Apply this change to a keyword collection.
    fn apply_keywords(&self, list: &mut KeywordCollection);
}
