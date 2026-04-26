use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use forge_carddb::CardDatabase;
use forge_engine_core::card::CardInstance;
use forge_engine_core::ids::PlayerId;
use memmap2::Mmap;

static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_IMAGE_MAP: OnceLock<HashMap<String, TokenImageInfo>> = OnceLock::new();

/// Scryfall set code + collector number for a token, derived from edition files.
#[derive(Debug, Clone)]
pub struct TokenImageInfo {
    /// Scryfall token set code (e.g., "thou" for Tokens of Hour of Devastation).
    pub set_code: String,
    /// Collector number within that token set (e.g., "1").
    pub collector_number: String,
}

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

/// Returns the path to the Forge editions directory.
fn editions_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("EDITIONS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../forge/forge-gui/res/editions")
    }
}

/// Returns the path to the pre-built cardset rkyv archive, if any.
/// Checks `CARDSET_ARCHIVE` env var first; falls back to `resources/cardset.rkyv`
/// adjacent to this crate's manifest (where the build script writes it).
fn cardset_archive_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARDSET_ARCHIVE") {
        PathBuf::from(path)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/cardset.rkyv")
    }
}

/// Returns the global CardDatabase, loading it on first call.
///
/// Prefers the pre-built rkyv archive at `cardset_archive_path()` (mmap'd,
/// zero-copy, ~6× faster cold start than the FS scan). Falls back to walking
/// `cards_dir()` directly when the archive is missing or invalid — which is
/// what `cargo tauri dev` does on a fresh checkout before `cargo build` has
/// produced the archive.
pub fn get_card_db() -> &'static CardDatabase {
    CARD_DB.get_or_init(|| {
        let archive_path = cardset_archive_path();
        let editions = editions_dir();
        if archive_path.exists() {
            match load_card_db_from_archive(&archive_path, &editions) {
                Ok((db, loaded, failed)) => {
                    eprintln!(
                        "[carddb] Loaded {} cards ({} failed) from archive {}",
                        loaded,
                        failed,
                        archive_path.display()
                    );
                    return db;
                }
                Err(err) => {
                    eprintln!(
                        "[carddb] Archive at {} unusable ({}); falling back to FS scan",
                        archive_path.display(),
                        err
                    );
                }
            }
        }

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

fn load_card_db_from_archive(
    archive_path: &std::path::Path,
    editions_dir: &std::path::Path,
) -> Result<(CardDatabase, usize, usize), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| format!("open: {e}"))?;
    let mmap = unsafe { Mmap::map(&file).map_err(|e| format!("mmap: {e}"))? };
    let editions_opt = if editions_dir.exists() {
        Some(editions_dir)
    } else {
        None
    };
    let (db, result) = CardDatabase::load_from_archive(&mmap, editions_opt)?;
    Ok((db, result.loaded, result.failed))
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

/// Returns the global token image mapping, built from edition files on first call.
///
/// Maps token script names (e.g., "w_1_1_spirit_flying") to their Scryfall
/// token set code and collector number (e.g., set="tmid", number="2").
///
/// Scryfall token sets use the convention: "t" + lowercase edition code.
/// For example, Hour of Devastation (HOU) has token set "thou".
pub fn get_token_image_map() -> &'static HashMap<String, TokenImageInfo> {
    TOKEN_IMAGE_MAP.get_or_init(|| {
        let dir = editions_dir();
        eprintln!(
            "[tokendb] Parsing edition files from {:?} for token images …",
            dir
        );
        let map = parse_edition_token_map(&dir);
        eprintln!("[tokendb] Built token image map with {} entries", map.len());
        map
    })
}

/// Parse all edition files to build a mapping from token script name
/// to (scryfall_token_set_code, collector_number).
fn parse_edition_token_map(editions_dir: &PathBuf) -> HashMap<String, TokenImageInfo> {
    let mut map = HashMap::new();

    let entries = match std::fs::read_dir(editions_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[tokendb] Failed to read editions dir: {}", err);
            return map;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract edition code: prefer ScryfallCode, fall back to Code
        let mut code: Option<String> = None;
        let mut scryfall_code: Option<String> = None;
        let mut in_tokens = false;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("[") {
                in_tokens = line == "[tokens]";
                continue;
            }

            if !in_tokens {
                if let Some(val) = line.strip_prefix("Code=") {
                    code = Some(val.trim().to_string());
                } else if let Some(val) = line.strip_prefix("ScryfallCode=") {
                    scryfall_code = Some(val.trim().to_string());
                }
                continue;
            }

            // Parse token line: "1 w_4_4_angel_flying @Adi Granov"
            // Format: <collector_number> <script_name> [@artist]
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 2 {
                continue;
            }

            let collector_number = parts[0].trim();
            // The script name may have trailing " @Artist" — strip it
            let script_name = parts[1].split(" @").next().unwrap_or(parts[1]).trim();

            if script_name.is_empty() || collector_number.is_empty() {
                continue;
            }

            // Build Scryfall token set code: "t" + lowercase(scryfall_code or code)
            let edition_code = scryfall_code.as_ref().or(code.as_ref());
            if let Some(ec) = edition_code {
                let token_set_code = format!("t{}", ec.to_lowercase());
                // Only insert if not already present — first edition wins,
                // ensuring we get a valid mapping without overwriting.
                map.entry(script_name.to_string())
                    .or_insert(TokenImageInfo {
                        set_code: token_set_code,
                        collector_number: collector_number.to_string(),
                    });
            }
        }
    }

    map
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
