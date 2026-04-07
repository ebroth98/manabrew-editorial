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
    /// Maps flavor-name aliases (lowercase) to canonical Oracle card names.
    flavor_name_aliases: HashMap<String, String>,
    /// Accent-stripped flavor-name aliases (lowercase) to canonical Oracle card names.
    flavor_name_aliases_normalized: HashMap<String, String>,
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
            flavor_name_aliases: HashMap::new(),
            flavor_name_aliases_normalized: HashMap::new(),
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
        // Mirror Java CardDb lookups: case-insensitive card names and
        // accent-stripped aliases should resolve to the same card.
        let resolved = self
            .normalized_names
            .get(card_name)
            .map(|s| s.as_str())
            .unwrap_or(card_name);
        let resolved = self.resolve_flavor_alias(resolved);

        self.cards
            .values()
            .find(|r| r.name().eq_ignore_ascii_case(resolved))
            .or_else(|| {
                let ascii_query = deunicode(resolved);
                self.cards.values().find(|r| {
                    let ascii_name = deunicode(&r.name());
                    ascii_name.eq_ignore_ascii_case(&ascii_query)
                })
            })
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
                    db.register_flavor_aliases_for_card(&card);

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
        let (mut db, result) = Self::load_from_strings(pairs);

        // Mirror Java CardDb behavior for flavor-name aliases sourced from edition data.
        // cardsfolder parent is expected to be ".../res", with edition files in ".../res/editions".
        if let Some(res_dir) = dir.parent() {
            let editions_dir = res_dir.join("editions");
            db.load_flavor_aliases_from_editions(&editions_dir);
        }

        (db, result)
    }

    fn resolve_flavor_alias<'a>(&'a self, name: &'a str) -> &'a str {
        if let Some(mapped) = self.flavor_name_aliases.get(&name.to_ascii_lowercase()) {
            return mapped;
        }
        let normalized = deunicode(name).to_ascii_lowercase();
        if let Some(mapped) = self.flavor_name_aliases_normalized.get(&normalized) {
            return mapped;
        }
        name
    }

    fn register_flavor_alias(&mut self, alias: &str, canonical_name: &str) {
        if alias.eq_ignore_ascii_case(canonical_name) {
            return;
        }
        self.flavor_name_aliases
            .insert(alias.to_ascii_lowercase(), canonical_name.to_string());

        let normalized_alias = deunicode(alias);
        if normalized_alias != alias {
            self.normalized_names
                .insert(normalized_alias.clone(), alias.to_string());
        }
        self.flavor_name_aliases_normalized.insert(
            normalized_alias.to_ascii_lowercase(),
            canonical_name.to_string(),
        );
    }

    fn register_flavor_aliases_for_card(&mut self, card: &CardRules) {
        let canonical = card.name();
        if let Some(alias) = &card.main_part.flavor_name {
            self.register_flavor_alias(alias, &canonical);
        }
        if let Some(other) = &card.other_part {
            if let Some(alias) = &other.flavor_name {
                self.register_flavor_alias(alias, &canonical);
            }
        }
        for face in card.specialized_parts.values() {
            if let Some(alias) = &face.flavor_name {
                self.register_flavor_alias(alias, &canonical);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_flavor_aliases_from_editions(&mut self, editions_dir: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(editions_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            self.extract_flavor_aliases_from_edition_contents(&contents);
        }
    }

    fn extract_flavor_aliases_from_edition_contents(&mut self, contents: &str) {
        let mut in_entries = false;

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1];
                in_entries = !section.eq_ignore_ascii_case("metadata");
                continue;
            }
            if !in_entries {
                continue;
            }
            if let Some((printed_name, flavor_name)) = parse_edition_flavor_alias_line(line) {
                let canonical = self
                    .get_by_card_name(&printed_name)
                    .map(|rules| rules.name())
                    .unwrap_or(printed_name);
                self.register_flavor_alias(&flavor_name, &canonical);
            }
        }
    }
}

fn parse_edition_flavor_alias_line(line: &str) -> Option<(String, String)> {
    let flavor_name = extract_flavor_name_json(line)?;
    let mut parts = line.splitn(3, char::is_whitespace);
    let _collector = parts.next()?;
    let _rarity = parts.next()?;
    let rest = parts.next()?.trim();
    let card_name = split_at_any(rest, &[" @", " ${"]).trim();
    if card_name.is_empty() {
        return None;
    }
    Some((card_name.to_string(), flavor_name))
}

fn split_at_any<'a>(input: &'a str, delimiters: &[&str]) -> &'a str {
    let mut best = input.len();
    for delim in delimiters {
        if let Some(idx) = input.find(delim) {
            best = best.min(idx);
        }
    }
    &input[..best]
}

fn extract_flavor_name_json(line: &str) -> Option<String> {
    let key_pos = line.find("\"flavorName\"")?;
    let tail = &line[key_pos + "\"flavorName\"".len()..];
    let colon = tail.find(':')?;
    let tail = &tail[colon + 1..].trim_start();
    if !tail.starts_with('"') {
        return None;
    }
    let value = &tail[1..];
    let end = value.find('"')?;
    Some(value[..end].to_string())
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
