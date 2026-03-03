use std::collections::HashMap;
use std::sync::OnceLock;

use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::card_db::{card_rules_to_instance, get_card_db};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardIdentity {
    pub name: String,
    pub set_code: String,
}

// ── Preset deck registry ───────────────────────────────────────────

/// Metadata for a preset deck, returned to the frontend via `get_preset_decks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDeckInfo {
    pub id: String,
    pub label: String,
    pub desc: String,
    /// Tailwind CSS text-color class used for the deck title in the UI.
    pub color: String,
}

// ── JSON deck file schema ──────────────────────────────────────────

/// A single card entry in a preset deck JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct DeckCardEntry {
    pub name: String,
    pub count: usize,
    #[serde(default)]
    pub set: String,
}

/// Full JSON schema for a preset deck file.
#[derive(Debug, Clone, Deserialize)]
struct PresetDeckFile {
    label: String,
    desc: String,
    color: String,
    #[serde(default)]
    opponent: Option<String>,
    #[serde(default)]
    ai_eligible: Option<bool>,
    #[serde(default)]
    order: Option<i32>,
    cards: Vec<DeckCardEntry>,
}

/// Loaded preset deck with its ID and parsed data.
#[derive(Debug, Clone)]
struct LoadedPreset {
    id: String,
    label: String,
    desc: String,
    color: String,
    opponent: Option<String>,
    ai_eligible: bool,
    order: i32,
    cards: Vec<DeckCardEntry>,
}

// ── Lazy-loaded deck registry ──────────────────────────────────────

/// Default directory for preset deck JSON files (relative to CWD).
const DEFAULT_DECKS_DIR: &str = "preset_decks";

static DECK_REGISTRY: OnceLock<Vec<LoadedPreset>> = OnceLock::new();

fn get_decks_dir() -> &'static str {
    // Allow override via environment variable for testing/deployment
    static DIR: OnceLock<String> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var("PRESET_DECKS_DIR").unwrap_or_else(|_| DEFAULT_DECKS_DIR.to_string())
    })
}

fn load_registry() -> Vec<LoadedPreset> {
    let dir = std::path::Path::new(get_decks_dir());
    let mut presets = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "[preset_decks] Failed to read directory '{}': {}",
                dir.display(),
                e
            );
            return presets;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[preset_decks] Failed to read '{}': {}", path.display(), e);
                continue;
            }
        };
        let deck: PresetDeckFile = match serde_json::from_str(&contents) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[preset_decks] Failed to parse '{}': {}", path.display(), e);
                continue;
            }
        };
        presets.push(LoadedPreset {
            id,
            label: deck.label,
            desc: deck.desc,
            color: deck.color,
            opponent: deck.opponent,
            ai_eligible: deck.ai_eligible.unwrap_or(false),
            order: deck.order.unwrap_or(999),
            cards: deck.cards,
        });
    }

    // Sort by order field to maintain deterministic UI ordering
    presets.sort_by_key(|p| (p.order, p.id.clone()));
    presets
}

fn get_registry() -> &'static Vec<LoadedPreset> {
    DECK_REGISTRY.get_or_init(load_registry)
}

fn get_preset_by_id(id: &str) -> Option<&'static LoadedPreset> {
    get_registry().iter().find(|p| p.id == id)
}

// ── Public API ─────────────────────────────────────────────────────

/// Return the ordered list of all available preset decks.
///
/// This is the single source of truth consumed by the `get_preset_decks`
/// Tauri command — the frontend no longer hardcodes deck names.
pub fn list_preset_decks() -> Vec<PresetDeckInfo> {
    get_registry()
        .iter()
        .map(|p| PresetDeckInfo {
            id: p.id.clone(),
            label: p.label.clone(),
            desc: p.desc.clone(),
            color: p.color.clone(),
        })
        .collect()
}

pub fn is_preset_id(id: &str) -> bool {
    get_registry().iter().any(|p| p.id == id)
}

/// Build decks for both players given a preset id.
///
/// Loads the preset for player 0 and the opponent deck (from the JSON's
/// `opponent` field) for player 1.
pub fn build_preset_decks(game: &mut GameState, preset_id: &str, p0: PlayerId, p1: PlayerId) {
    let preset = match get_preset_by_id(preset_id) {
        Some(p) => p,
        None => {
            // Fallback to red_burn
            if let Some(rb) = get_preset_by_id("red_burn") {
                build_deck_from_entries(game, p0, &rb.cards);
            }
            if let Some(gs) = get_preset_by_id("green_stompy") {
                build_deck_from_entries(game, p1, &gs.cards);
            }
            return;
        }
    };

    build_deck_from_entries(game, p0, &preset.cards);

    // Determine opponent deck
    match preset.opponent.as_deref() {
        Some("random") => {
            let ai_cards = random_ai_deck_cards();
            build_deck_from_entries(game, p1, ai_cards);
        }
        Some(opp_id) => {
            if let Some(opp) = get_preset_by_id(opp_id) {
                build_deck_from_entries(game, p1, &opp.cards);
            } else {
                eprintln!("[preset_decks] Unknown opponent '{}', using red_burn", opp_id);
                if let Some(rb) = get_preset_by_id("red_burn") {
                    build_deck_from_entries(game, p1, &rb.cards);
                }
            }
        }
        None => {
            // No opponent specified, use green_stompy as default
            if let Some(gs) = get_preset_by_id("green_stompy") {
                build_deck_from_entries(game, p1, &gs.cards);
            }
        }
    }
}

/// Build a single-player deck for `owner` from a preset id.
/// Falls back to the red burn preset if `preset_id` is unknown.
pub fn build_preset_deck_for_player(game: &mut GameState, preset_id: &str, owner: PlayerId) {
    let preset = get_preset_by_id(preset_id)
        .or_else(|| get_preset_by_id("red_burn"));
    if let Some(p) = preset {
        build_deck_from_entries(game, owner, &p.cards);
    }
}

/// Pick a random deck from all AI-eligible presets.
fn random_ai_deck_cards() -> &'static Vec<DeckCardEntry> {
    let registry = get_registry();
    let ai_eligible: Vec<&LoadedPreset> = registry.iter().filter(|p| p.ai_eligible).collect();
    let mut rng = rand::thread_rng();
    ai_eligible
        .choose(&mut rng)
        .map(|p| &p.cards)
        .unwrap_or_else(|| {
            // Fallback: first deck in registry
            &registry[0].cards
        })
}

// ── Deck builders ──────────────────────────────────────────────────

/// Build the default AI opponent deck (random AI-eligible) for a single player.
///
/// Used when the human plays a custom deck so the AI still gets a deck.
pub fn build_ai_opponent(game: &mut GameState, owner: PlayerId) {
    let cards = random_ai_deck_cards();
    build_deck_from_entries(game, owner, cards);
}

/// Build a deck from a list of DeckCardEntry, loading each card definition
/// from the global CardDatabase (parsed from the Forge card scripts).
/// The set code is stored on each card instance so the UI can request the
/// specific printing from Scryfall. An empty set code means no preference.
fn build_deck_from_entries(game: &mut GameState, owner: PlayerId, deck: &[DeckCardEntry]) {
    let db = get_card_db();
    for entry in deck {
        match db.get_by_card_name(&entry.name) {
            Some(rules) => {
                for _ in 0..entry.count {
                    let mut card = card_rules_to_instance(rules, owner);
                    if !entry.set.is_empty() {
                        card.set_code = Some(entry.set.clone());
                    }
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => eprintln!("[deck] Unknown card '{}' — skipped", entry.name),
        }
    }
}

/// Build a custom deck for `owner` from a list of card identities (one per
/// copy), loading each definition from the global CardDatabase.
/// Unrecognised names are skipped with a log message.
pub fn build_custom_deck(game: &mut GameState, owner: PlayerId, identities: &[CardIdentity]) {
    let db = get_card_db();
    for identity in identities {
        let name = &identity.name;
        match db.get_by_card_name(name) {
            Some(rules) => {
                let mut card = card_rules_to_instance(rules, owner);
                if !identity.set_code.is_empty() {
                    card.set_code = Some(identity.set_code.clone());
                }
                let id = game.create_card(card);
                game.move_card(id, ZoneType::Library, owner);
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", name),
        }
    }
}
