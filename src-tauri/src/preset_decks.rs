use std::sync::OnceLock;

use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_engine_core::player::RegisteredPlayer;
use forge_foundation::CoreType;
use forge_foundation::ZoneType;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::card_db::{card_rules_to_instance, get_card_db};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardIdentity {
    pub name: String,
    pub set_code: String,
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedRegisteredPlayer {
    pub registered: RegisteredPlayer,
    pub cards: Vec<(CardInstance, ZoneType)>,
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

/// Default directory name for preset deck JSON files at repo root.
const DEFAULT_DECKS_DIR: &str = "preset_decks";

static DECK_REGISTRY: OnceLock<Vec<LoadedPreset>> = OnceLock::new();

fn get_decks_dir() -> &'static std::path::PathBuf {
    // Resolve path relative to this crate (`src-tauri`) instead of process CWD.
    static DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let configured =
            std::env::var("PRESET_DECKS_DIR").unwrap_or_else(|_| DEFAULT_DECKS_DIR.to_string());
        let configured_path = std::path::PathBuf::from(configured);
        if configured_path.is_absolute() {
            configured_path
        } else {
            project_root.join(configured_path)
        }
    })
}

fn load_registry() -> Vec<LoadedPreset> {
    let dir = get_decks_dir();
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

    ensure_registry_invariants(&mut presets);

    // Sort by order field to maintain deterministic UI ordering
    presets.sort_by_key(|p| (p.order, p.id.clone()));
    presets
}

fn fallback_preset(
    id: &str,
    label: &str,
    desc: &str,
    color: &str,
    opponent: Option<&str>,
    ai_eligible: bool,
    basic_land_name: &str,
    order: i32,
) -> LoadedPreset {
    LoadedPreset {
        id: id.to_string(),
        label: label.to_string(),
        desc: desc.to_string(),
        color: color.to_string(),
        opponent: opponent.map(str::to_string),
        ai_eligible,
        order,
        cards: vec![DeckCardEntry {
            name: basic_land_name.to_string(),
            count: 60,
            set: String::new(),
        }],
    }
}

fn ensure_registry_invariants(presets: &mut Vec<LoadedPreset>) {
    if presets.is_empty() {
        eprintln!("[preset_decks] No decks loaded from JSON. Injecting fallback presets.");
    }

    if !presets.iter().any(|p| p.id == "red_burn") {
        eprintln!("[preset_decks] Missing 'red_burn'. Injecting fallback preset.");
        presets.push(fallback_preset(
            "red_burn",
            "Red Burn (Fallback)",
            "Failsafe deck used when preset JSON loading fails",
            "text-red-500",
            Some("green_stompy"),
            true,
            "Mountain",
            10_000,
        ));
    }

    if !presets.iter().any(|p| p.id == "green_stompy") {
        eprintln!("[preset_decks] Missing 'green_stompy'. Injecting fallback preset.");
        presets.push(fallback_preset(
            "green_stompy",
            "Green Stompy (Fallback)",
            "Failsafe deck used when preset JSON loading fails",
            "text-green-500",
            Some("red_burn"),
            true,
            "Forest",
            10_001,
        ));
    }

    if !presets.iter().any(|p| p.ai_eligible) {
        eprintln!("[preset_decks] No AI-eligible decks. Marking 'red_burn' fallback as eligible.");
        if let Some(rb) = presets.iter_mut().find(|p| p.id == "red_burn") {
            rb.ai_eligible = true;
        }
    }
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
#[allow(dead_code)]
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
                eprintln!(
                    "[preset_decks] Unknown opponent '{}', using red_burn",
                    opp_id
                );
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
    let preset = get_preset_by_id(preset_id).or_else(|| get_preset_by_id("red_burn"));
    if let Some(p) = preset {
        build_deck_from_entries(game, owner, &p.cards);
    }
}

pub fn prepare_preset_registered_player(
    name: impl Into<String>,
    preset_id: &str,
) -> PreparedRegisteredPlayer {
    let mut registered = RegisteredPlayer::new(name);
    let preset = get_preset_by_id(preset_id).or_else(|| get_preset_by_id("red_burn"));
    let cards = preset
        .map(|preset| prepare_cards_from_entries(&preset.cards, &mut registered))
        .unwrap_or_default();
    PreparedRegisteredPlayer { registered, cards }
}

/// Build the opponent deck configured for a given preset id.
///
/// Reads the preset's `opponent` field and loads that deck for `owner`.
/// Falls back to a random AI-eligible deck if the preset or opponent is missing.
pub fn build_preset_opponent(game: &mut GameState, preset_id: &str, owner: PlayerId) {
    let preset = get_preset_by_id(preset_id);
    match preset.and_then(|p| p.opponent.as_deref()) {
        Some("random") => {
            let ai_cards = random_ai_deck_cards();
            build_deck_from_entries(game, owner, ai_cards);
        }
        Some(opp_id) => {
            if let Some(opp) = get_preset_by_id(opp_id) {
                build_deck_from_entries(game, owner, &opp.cards);
            } else {
                eprintln!(
                    "[preset_decks] Unknown opponent '{}', using random AI deck",
                    opp_id
                );
                let ai_cards = random_ai_deck_cards();
                build_deck_from_entries(game, owner, ai_cards);
            }
        }
        None => {
            let ai_cards = random_ai_deck_cards();
            build_deck_from_entries(game, owner, ai_cards);
        }
    }
}

pub fn prepare_preset_opponent_registered_player(
    name: impl Into<String>,
    preset_id: &str,
) -> PreparedRegisteredPlayer {
    match get_preset_by_id(preset_id).and_then(|p| p.opponent.as_deref()) {
        Some("random") => prepare_ai_registered_player(name),
        Some(opp_id) => {
            if get_preset_by_id(opp_id).is_some() {
                prepare_preset_registered_player(name, opp_id)
            } else {
                eprintln!(
                    "[preset_decks] Unknown opponent '{}', using random AI deck",
                    opp_id
                );
                prepare_ai_registered_player(name)
            }
        }
        None => prepare_ai_registered_player(name),
    }
}

/// Pick a random deck from all AI-eligible presets.
fn random_ai_deck_cards() -> &'static Vec<DeckCardEntry> {
    static EMPTY_DECK: OnceLock<Vec<DeckCardEntry>> = OnceLock::new();
    let registry = get_registry();
    let ai_eligible: Vec<&LoadedPreset> = registry.iter().filter(|p| p.ai_eligible).collect();
    let mut rng = rand::thread_rng();
    ai_eligible
        .choose(&mut rng)
        .map(|p| &p.cards)
        .unwrap_or_else(|| {
            // Fallback: first deck in registry, otherwise empty deck to avoid panics.
            if let Some(first) = registry.first() {
                &first.cards
            } else {
                eprintln!(
                    "[preset_decks] Registry is empty; AI opponent will receive an empty deck"
                );
                EMPTY_DECK.get_or_init(Vec::new)
            }
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

pub fn prepare_ai_registered_player(name: impl Into<String>) -> PreparedRegisteredPlayer {
    let mut registered = RegisteredPlayer::new(name);
    let cards = prepare_cards_from_entries(random_ai_deck_cards(), &mut registered);
    PreparedRegisteredPlayer { registered, cards }
}

/// Build a deck from a list of DeckCardEntry, loading each card definition
/// from the global CardDatabase (parsed from the Forge card scripts).
/// The set code is stored on each card instance so the UI can request the
/// specific printing from Scryfall. An empty set code means no preference.
fn build_deck_from_entries(game: &mut GameState, owner: PlayerId, deck: &[DeckCardEntry]) {
    let db = get_card_db();
    for entry in deck {
        match lookup_card_rules(db, &entry.name) {
            Some(rules) => {
                for _ in 0..entry.count {
                    let mut card = card_rules_to_instance(rules, owner);
                    if !entry.set.is_empty() {
                        card.set_code = Some(entry.set.clone());
                    }
                    let destination = fallback_deck_zone_for_card(&card);
                    let id = game.create_card(card);
                    game.move_card(id, destination, owner);
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
        match lookup_card_rules(db, name) {
            Some(rules) => {
                let mut card = card_rules_to_instance(rules, owner);
                if !identity.set_code.is_empty() {
                    card.set_code = Some(identity.set_code.clone());
                }
                let destination = deck_zone_for_identity(identity, &card);
                let id = game.create_card(card);
                if destination == ZoneType::Command {
                    game.card_mut(id).is_commander = true;
                }
                game.move_card(id, destination, owner);
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", name),
        }
    }
}

pub fn prepare_custom_registered_player(
    name: impl Into<String>,
    identities: &[CardIdentity],
) -> PreparedRegisteredPlayer {
    let mut registered = RegisteredPlayer::new(name);
    let cards = prepare_cards_from_identities(identities, &mut registered);
    PreparedRegisteredPlayer { registered, cards }
}

fn prepare_cards_from_entries(
    deck: &[DeckCardEntry],
    registered: &mut RegisteredPlayer,
) -> Vec<(CardInstance, ZoneType)> {
    let db = get_card_db();
    let mut cards = Vec::new();
    for entry in deck {
        match lookup_card_rules(db, &entry.name) {
            Some(rules) => {
                for _ in 0..entry.count {
                    let mut card = card_rules_to_instance(rules, PlayerId(0));
                    if !entry.set.is_empty() {
                        card.set_code = Some(entry.set.clone());
                    }
                    let destination = fallback_deck_zone_for_card(&card);
                    register_card_name(registered, &card.card_name, destination);
                    cards.push((card, destination));
                }
            }
            None => eprintln!("[deck] Unknown card '{}' — skipped", entry.name),
        }
    }
    cards
}

fn prepare_cards_from_identities(
    identities: &[CardIdentity],
    registered: &mut RegisteredPlayer,
) -> Vec<(CardInstance, ZoneType)> {
    let db = get_card_db();
    let mut cards = Vec::new();
    for identity in identities {
        match lookup_card_rules(db, &identity.name) {
            Some(rules) => {
                let mut card = card_rules_to_instance(rules, PlayerId(0));
                if !identity.set_code.is_empty() {
                    card.set_code = Some(identity.set_code.clone());
                }
                let destination = deck_zone_for_identity(identity, &card);
                register_card_name(registered, &card.card_name, destination);
                cards.push((card, destination));
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", identity.name),
        }
    }
    cards
}

fn register_card_name(registered: &mut RegisteredPlayer, card_name: &str, destination: ZoneType) {
    let name = card_name.to_string();
    match destination {
        ZoneType::Library => {
            registered.original_deck.push(name.clone());
            registered.current_deck.push(name);
        }
        ZoneType::Command => registered.commanders.push(name),
        ZoneType::Battlefield => registered.cards_on_battlefield.push(name),
        ZoneType::SchemeDeck => registered.schemes.push(name),
        ZoneType::PlanarDeck => registered.planes.push(name),
        ZoneType::AttractionDeck => registered.attractions.push(name),
        ZoneType::ContraptionDeck => registered.contraptions.push(name),
        ZoneType::Sideboard => {}
        _ => {}
    }
}

fn lookup_card_rules<'a>(
    db: &'a forge_carddb::CardDatabase,
    raw_name: &str,
) -> Option<&'a forge_carddb::CardRules> {
    db.get_by_card_name(raw_name).or_else(|| {
        raw_name
            .split_once(" // ")
            .and_then(|(front_face, _)| db.get_by_card_name(front_face.trim()))
    })
}

fn fallback_deck_zone_for_card(card: &forge_engine_core::card::Card) -> ZoneType {
    if card
        .type_line
        .subtypes
        .iter()
        .any(|subtype| subtype.eq_ignore_ascii_case("Attraction"))
    {
        ZoneType::AttractionDeck
    } else if card
        .type_line
        .subtypes
        .iter()
        .any(|subtype| subtype.eq_ignore_ascii_case("Contraption"))
    {
        ZoneType::ContraptionDeck
    } else if card.type_line.core_types.contains(&CoreType::Scheme) {
        ZoneType::SchemeDeck
    } else if card.type_line.core_types.contains(&CoreType::Plane) {
        ZoneType::PlanarDeck
    } else {
        ZoneType::Library
    }
}

fn deck_zone_for_identity(
    identity: &CardIdentity,
    card: &forge_engine_core::card::Card,
) -> ZoneType {
    match identity.section.as_deref() {
        Some("main") => ZoneType::Library,
        Some("sideboard") => ZoneType::Sideboard,
        Some("commander") => ZoneType::Command,
        Some("attractions") => ZoneType::AttractionDeck,
        Some("contraptions") => ZoneType::ContraptionDeck,
        Some("schemes") => ZoneType::SchemeDeck,
        Some("planes") => ZoneType::PlanarDeck,
        _ => fallback_deck_zone_for_card(card),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_engine_core::card::Card;
    use forge_engine_core::ids::CardId;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    #[test]
    fn routes_variant_cards_to_variant_decks() {
        let owner = PlayerId(0);
        let attraction = Card::new(
            CardId(0),
            "Balloon Stand".to_string(),
            owner,
            CardTypeLine::parse("Artifact Attraction"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        let contraption = Card::new(
            CardId(0),
            "Auto-Key".to_string(),
            owner,
            CardTypeLine::parse("Artifact Contraption"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        let normal = Card::new(
            CardId(0),
            "Forest".to_string(),
            owner,
            CardTypeLine::parse("Basic Land Forest"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );

        assert_eq!(
            fallback_deck_zone_for_card(&attraction),
            ZoneType::AttractionDeck
        );
        assert_eq!(
            fallback_deck_zone_for_card(&contraption),
            ZoneType::ContraptionDeck
        );
        assert_eq!(fallback_deck_zone_for_card(&normal), ZoneType::Library);
    }

    #[test]
    fn explicit_section_overrides_fallback_routing() {
        let owner = PlayerId(0);
        let normal = Card::new(
            CardId(0),
            "Forest".to_string(),
            owner,
            CardTypeLine::parse("Basic Land Forest"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        assert_eq!(
            deck_zone_for_identity(
                &CardIdentity {
                    name: "Forest".to_string(),
                    set_code: "".to_string(),
                    section: Some("sideboard".to_string()),
                },
                &normal,
            ),
            ZoneType::Sideboard
        );
        assert_eq!(
            deck_zone_for_identity(
                &CardIdentity {
                    name: "Forest".to_string(),
                    set_code: "".to_string(),
                    section: Some("commander".to_string()),
                },
                &normal,
            ),
            ZoneType::Command
        );
    }
}
