use std::path::PathBuf;
use std::sync::OnceLock;

use forge_carddb::{CardDatabase, CardRules};
use forge_engine_core::card::CardInstance;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
use forge_engine_core::trigger::parse_trigger;

static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();

/// Returns the path to the Forge card scripts directory.
/// Checks the CARDS_DIR env var first; falls back to the path adjacent
/// to this crate's manifest (works during `cargo tauri dev`).
fn cards_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARDS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../forge/forge-gui/res/cardsfolder")
    }
}

/// Returns the path to the Forge token scripts directory.
fn token_scripts_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TOKEN_SCRIPTS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../forge/forge-gui/res/tokenscripts")
    }
}

/// Returns the global CardDatabase, loading it on first call.
///
/// Loads all card scripts from the Forge cardsfolder — the same source of
/// truth used by the Java Forge engine. No card data is hardcoded here.
pub fn get_card_db() -> &'static CardDatabase {
    CARD_DB.get_or_init(|| {
        let dir = cards_dir();
        eprintln!("[carddb] Loading cards from {:?} …", dir);
        let (db, result) = CardDatabase::load_from_directory(&dir);
        eprintln!(
            "[carddb] Loaded {} cards ({} failed)",
            result.loaded, result.failed
        );
        if !result.errors.is_empty() {
            for (file, err) in result.errors.iter().take(10) {
                eprintln!("[carddb]   parse error in {}: {}", file, err);
            }
        }
        db
    })
}

/// Returns the global token-script database, loading it on first call.
///
/// Token scripts live in `forge/forge-gui/res/tokenscripts/` and are keyed
/// by their filename stem (e.g. "r_1_1_goblin" for `r_1_1_goblin.txt`).
pub fn get_token_db() -> &'static CardDatabase {
    TOKEN_DB.get_or_init(|| {
        let dir = token_scripts_dir();
        eprintln!("[tokendb] Loading token scripts from {:?} …", dir);
        let (db, result) = CardDatabase::load_from_directory(&dir);
        eprintln!(
            "[tokendb] Loaded {} token scripts ({} failed)",
            result.loaded, result.failed
        );
        db
    })
}

/// Convert an immutable `CardRules` definition into a mutable `CardInstance`
/// ready to be inserted into a game.
///
/// Mirrors Java's `CardFactory.readCard()` + `readCardFace()`:
/// - copies base stats (name, mana cost, type line, color, P/T, keywords,
///   raw ability strings)
/// - parses `T:` trigger strings into `Trigger` structs via `parse_trigger()`
/// - copies SVars into `card.svars` (must be present before triggers fire)
///
/// The `CardId` inside the returned instance is a placeholder (0); the real
/// ID is assigned by `game.create_card()`.
pub fn card_rules_to_instance(rules: &CardRules, owner: PlayerId) -> CardInstance {
    let face = &rules.main_part;
    let mut next_trigger_id = 0u32;

    // Parse each raw trigger string (T: line) into a Trigger struct.
    // Unknown/unsupported trigger modes return None and are skipped with a warning.
    let triggers: Vec<_> = face
        .triggers
        .iter()
        .filter_map(|raw| {
            let result = parse_trigger(raw, &mut next_trigger_id);
            if result.is_none() {
                eprintln!(
                    "[carddb] Unsupported trigger on '{}': {:?} — skipped",
                    face.name, raw
                );
            }
            result
        })
        .collect();

    let mut card = CardInstance::new(
        CardId(0),
        face.name.clone(),
        owner,
        face.type_line.clone(),
        face.mana_cost.clone(),
        face.resolved_color(),
        face.int_power,
        face.int_toughness,
        face.keywords.clone(),
        face.abilities.clone(),
    );

    card.triggers = triggers;
    // SVars must be copied so trigger Execute$ references resolve correctly.
    card.svars = face.svars.clone();

    // Load static abilities from S: lines (stored separately from A: ability lines
    // in Forge card scripts).  The parser strips the "S:" key and stores only the
    // value, so we re-prefix with "S$ " to match parse_static_ability's format.
    for raw in &face.static_abilities {
        let prefixed = format!("S$ {}", raw);
        if let Some(sa) = parse_static_ability(&prefixed) {
            card.static_abilities.push(sa);
        }
    }

    // Load replacement effects from R: lines in the same way.
    for raw in &face.replacements {
        let prefixed = format!("R$ {}", raw);
        if let Some(re) = parse_replacement_effect(&prefixed) {
            card.replacement_effects.push(re);
        }
    }

    card
}
