use std::path::PathBuf;
use std::sync::OnceLock;

use forge_carddb::CardDatabase;
use forge_engine_core::card::CardInstance;
use forge_engine_core::ids::PlayerId;

static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();

/// Returns the path to the Forge card scripts directory.
/// Checks the CARDS_DIR env var first; falls back to the path adjacent
/// to this crate's manifest (works during `cargo tauri dev`).
fn cards_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARDS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../forge/forge-gui/res/cardsfolder")
    }
}

/// Returns the path to the Forge token scripts directory.
fn token_scripts_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TOKEN_SCRIPTS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../forge/forge-gui/res/tokenscripts")
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
/// Delegates to `CardInstance::from_rules()` which mirrors Java's
/// `CardFactory.readCard()` + `CardFactoryUtil` intrinsic ability generation.
///
/// The `CardId` inside the returned instance is a placeholder (0); the real
/// ID is assigned by `game.create_card()`.
pub fn card_rules_to_instance(rules: &forge_carddb::CardRules, owner: PlayerId) -> CardInstance {
    CardInstance::from_rules(rules, owner)
}
