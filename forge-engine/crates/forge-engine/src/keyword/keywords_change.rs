//! Tracks keyword additions and removals.
//!
//! Ported from Java's `KeywordsChange.java` in `forge/game/keyword/`.

use super::keyword_collection::KeywordCollection;
use super::keyword_instance::KeywordInstanceData;
use super::trait_keywords_change::KeywordsChange as IKeywordsChange;

/// Tracks a set of keyword changes (additions and removals).
/// Mirrors Java's `KeywordsChange` class.
#[derive(Debug, Clone, Default)]
pub struct KeywordsChange {
    /// Keywords to add.
    pub keywords: KeywordCollection,
    /// Keywords to remove by original string.
    pub remove_keywords: Vec<String>,
    /// Keywords to remove by instance.
    pub remove_keyword_instances: Vec<KeywordInstanceData>,
    /// Whether to remove all existing keywords.
    pub remove_all_keywords: bool,
}

impl KeywordsChange {
    /// Create a new keywords change.
    pub fn new(
        keyword_list: Option<Vec<KeywordInstanceData>>,
        remove_keyword_list: Option<Vec<String>>,
        remove_all: bool,
    ) -> Self {
        let mut keywords = KeywordCollection::new();
        if let Some(list) = keyword_list {
            for inst in list {
                keywords.insert(inst);
            }
        }

        let remove_keywords = remove_keyword_list.unwrap_or_default();

        Self {
            keywords,
            remove_keywords,
            remove_keyword_instances: Vec::new(),
            remove_all_keywords: remove_all,
        }
    }

    /// Get the keywords to add.
    pub fn get_keywords(&self) -> Vec<&KeywordInstanceData> {
        self.keywords.get_values()
    }

    /// Get the keywords to remove by string.
    pub fn get_remove_keywords(&self) -> &[String] {
        &self.remove_keywords
    }

    /// Whether to remove all keywords.
    pub fn is_remove_all_keywords(&self) -> bool {
        self.remove_all_keywords
    }

    /// Whether this change has no effect.
    pub fn is_empty(&self) -> bool {
        !self.remove_all_keywords
            && self.keywords.is_empty()
            && self.remove_keywords.is_empty()
            && self.remove_keyword_instances.is_empty()
    }
}

impl IKeywordsChange for KeywordsChange {
    fn apply_keywords(&self, list: &mut KeywordCollection) {
        if self.remove_all_keywords {
            list.clear();
        } else {
            let strs: Vec<&str> = self.remove_keywords.iter().map(|s| s.as_str()).collect();
            list.remove_strings(strs);
        }

        // Remove specific instances
        for inst in &self.remove_keyword_instances {
            list.remove(&inst.original);
        }

        // Add new keywords
        for inst in self.keywords.get_values() {
            list.insert(inst.clone());
        }
    }
}
