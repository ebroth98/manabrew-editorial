use std::collections::HashMap;

use deunicode::deunicode;

use crate::card_rules::CardRules;
use crate::parser::CardScriptParser;

/// A loaded collection of card definitions.
/// Cards are keyed by their normalized name (filename without extension).
#[derive(Debug)]
pub struct CardDatabase {
    cards: HashMap<String, CardRules>,
    /// Maps accent-stripped names to original names (mirrors Java's normalizedNames).
    normalized_names: HashMap<String, String>,
}

/// Result of loading a batch of card scripts.
#[derive(Debug, Default)]
pub struct LoadResult {
    pub loaded: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>,
}

impl CardDatabase {
    pub fn new() -> Self {
        CardDatabase {
            cards: HashMap::new(),
            normalized_names: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&CardRules> {
        self.cards.get(name)
    }

    pub fn get_by_card_name(&self, card_name: &str) -> Option<&CardRules> {
        // Mirror Java's getNormalizedName: check if this is an accent-stripped name
        let resolved = self
            .normalized_names
            .get(card_name)
            .map(|s| s.as_str())
            .unwrap_or(card_name);
        self.cards.values().find(|r| r.name() == resolved)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &CardRules)> {
        self.cards.iter()
    }

    pub fn card_names(&self) -> impl Iterator<Item = String> + '_ {
        self.cards.values().map(|r| r.name())
    }

    /// Mirror of Java's CardDb.getNormalizedName().
    /// If the given name is an accent-stripped variant, returns the original name.
    pub fn get_normalized_name<'a>(&'a self, card_name: &'a str) -> &'a str {
        self.normalized_names
            .get(card_name)
            .map(|s| s.as_str())
            .unwrap_or(card_name)
    }

    /// Load cards from an iterator of (filename, script_content) pairs.
    /// This is the WASM-compatible entry point — no filesystem access.
    pub fn load_from_strings<'a, I>(scripts: I) -> (Self, LoadResult)
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut db = CardDatabase::new();
        let mut result = LoadResult::default();
        let mut parser = CardScriptParser::new();

        for (filename, content) in scripts {
            let lines: Vec<&str> = content.lines().collect();
            match parser.parse(lines, Some(filename)) {
                Ok(card) => {
                    let key = card.normalized_name.clone();
                    let key = if key.is_empty() { card.name() } else { key };

                    // Mirror Java's addFaceToDbNames:
                    // final String normalName = StringUtils.stripAccents(name);
                    // if (!normalName.equals(name)) {
                    //     normalizedNames.put(normalName, name);
                    // }
                    let card_name = card.name();
                    let normal_name = deunicode(&card_name);
                    if normal_name != card_name {
                        db.normalized_names.insert(normal_name, card_name);
                    }

                    db.cards.insert(key, card);
                    result.loaded += 1;
                }
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((filename.to_string(), e));
                }
            }
        }

        (db, result)
    }

    /// Load cards from a directory on the filesystem.
    /// Walks the directory recursively looking for .txt files.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_directory(dir: &std::path::Path) -> (Self, LoadResult) {
        let mut scripts = Vec::new();

        if let Ok(entries) = collect_txt_files(dir) {
            for path in entries {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let filename = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    scripts.push((filename, content));
                }
            }
        }

        let pairs: Vec<(&str, &str)> = scripts
            .iter()
            .map(|(f, c)| (f.as_str(), c.as_str()))
            .collect();
        Self::load_from_strings(pairs)
    }
}

impl Default for CardDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively collect all .txt files in a directory.
#[cfg(not(target_arch = "wasm32"))]
fn collect_txt_files(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut results = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                results.extend(collect_txt_files(&path)?);
            } else if path.extension().and_then(|e| e.to_str()) == Some("txt") {
                results.push(path);
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_strings() {
        let scripts = vec![
            (
                "lightning_bolt",
                "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\nOracle:Bolt!",
            ),
            (
                "grizzly_bears",
                "Name:Grizzly Bears\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2\nOracle:",
            ),
        ];

        let (db, result) = CardDatabase::load_from_strings(scripts);
        assert_eq!(result.loaded, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(db.len(), 2);
        assert!(db.get("lightning_bolt").is_some());
        assert!(db.get("grizzly_bears").is_some());
    }

    #[test]
    fn get_by_card_name() {
        let scripts = vec![(
            "lightning_bolt",
            "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\nOracle:Bolt!",
        )];

        let (db, _) = CardDatabase::load_from_strings(scripts);
        let card = db.get_by_card_name("Lightning Bolt").unwrap();
        assert_eq!(card.main_part.name, "Lightning Bolt");
    }

    #[test]
    fn get_by_card_name_accent_normalized() {
        let scripts = vec![(
            "troll_of_khazad_dum",
            "Name:Troll of Khazad-d\u{00fb}m\nManaCost:5 B\nTypes:Creature Troll\nPT:6/5\nOracle:Swampwalk",
        )];

        let (db, _) = CardDatabase::load_from_strings(scripts);
        // Exact match should work
        assert!(db.get_by_card_name("Troll of Khazad-d\u{00fb}m").is_some());
        // ASCII-stripped name should also work (via normalized_names map)
        assert!(db.get_by_card_name("Troll of Khazad-dum").is_some());
    }
}
