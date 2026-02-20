use std::path::PathBuf;
use std::sync::OnceLock;

use forge_carddb::{CardDatabase, CardRules};
use forge_engine_core::card::CardInstance;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::trigger::parse_trigger;

static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();

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

    card
}
