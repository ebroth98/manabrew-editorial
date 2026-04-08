use crate::deck_generator;
use forge_carddb::CardDatabase;
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DeckCardEntry {
    name: String,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct PresetDeckFile {
    cards: Vec<DeckCardEntry>,
}

fn load_preset_deck(name: &str, decks_dir: &str) -> Result<Vec<(String, usize)>, String> {
    let path = std::path::Path::new(decks_dir).join(format!("{}.json", name));
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read preset deck '{}': {}", path.display(), e))?;
    let deck: PresetDeckFile = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))?;
    Ok(deck.cards.into_iter().map(|c| (c.name, c.count)).collect())
}

pub fn available_presets(decks_dir: &str) -> Vec<String> {
    let dir = std::path::Path::new(decks_dir);
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}
pub fn resolve_deck_spec(spec: &str, decks_dir: &str) -> Result<Vec<(String, usize)>, String> {
    if let Some(inline) = spec.strip_prefix("inline:") {
        deck_generator::parse_inline(inline)
    } else if let Some(path) = spec.strip_prefix("file:") {
        parse_deck_file(path)
    } else {
        load_preset_deck(spec, decks_dir)
    }
}

fn parse_deck_file(path: &str) -> Result<Vec<(String, usize)>, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let mut deck = Vec::new();
    for (line_num, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on first whitespace: "4 Lightning Bolt" -> ("4", "Lightning Bolt")
        let (count_str, name) = line.split_once(char::is_whitespace).ok_or_else(|| {
            format!(
                "Line {}: expected 'Count CardName', got '{}'",
                line_num + 1,
                line
            )
        })?;
        let count: usize = count_str.trim().parse().map_err(|_| {
            format!(
                "Line {}: invalid count '{}' in '{}'",
                line_num + 1,
                count_str,
                line
            )
        })?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!(
                "Line {}: empty card name in '{}'",
                line_num + 1,
                line
            ));
        }
        deck.push((name.to_string(), count));
    }
    if deck.is_empty() {
        return Err(format!("Deck file '{}' contains no cards", path));
    }
    Ok(deck)
}
pub fn build_deck_from_spec(
    game: &mut GameState,
    db: &CardDatabase,
    owner: PlayerId,
    spec: &[(String, usize)],
    verbose: bool,
) {
    for (name, count) in spec {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let card = CardInstance::from_rules(rules, owner);
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => {
                if verbose {
                    eprintln!("[parity] Unknown card '{}' — skipped", name);
                }
            }
        }
    }
}
