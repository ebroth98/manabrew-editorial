//! Card database loading for WASM.
//!
//! This module handles loading card data from a JSON bundle that's fetched
//! by the web worker.

use std::collections::HashMap;
use std::sync::OnceLock;

use forge_carddb::CardDatabase;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// The global card database, loaded once from a JSON bundle.
static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
/// The global token-script database, loaded once from a JSON bundle.
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();

/// JSON structure for the card bundle.
#[derive(Debug, Deserialize)]
pub struct CardBundle {
    pub version: u32,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub cards: HashMap<String, String>,
}

/// A preset deck from the JSON bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDeck {
    pub id: String,
    pub label: String,
    pub desc: String,
    pub color: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commander: Option<String>,
    pub cards: Vec<DeckCard>,
}

fn default_format() -> String {
    "standard".to_string()
}

/// A card entry in a preset deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCard {
    pub name: String,
    pub count: usize,
    #[serde(default)]
    pub set: String,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Load the card database from a JSON bundle string.
///
/// This should be called once at startup with the contents of cards-bundle.json.
/// Returns the number of cards loaded.
#[wasm_bindgen]
pub fn load_card_bundle(json_str: &str) -> Result<u32, JsError> {
    // Parse the JSON bundle
    let bundle: CardBundle = serde_json::from_str(json_str)
        .map_err(|e| JsError::new(&format!("Failed to parse card bundle: {}", e)))?;

    web_sys::console::log_1(
        &format!(
            "[card_loader] Loading {} cards from bundle (v{})",
            bundle.cards.len(),
            bundle.version
        )
        .into(),
    );

    // Convert to the format expected by CardDatabase::load_from_strings
    let scripts: Vec<(&str, &str)> = bundle
        .cards
        .iter()
        .map(|(filename, content)| (filename.as_str(), content.as_str()))
        .collect();

    // Load into the database
    let (db, result) = CardDatabase::load_from_strings(scripts);

    if result.failed > 0 {
        web_sys::console::warn_1(
            &format!("[card_loader] {} cards failed to parse", result.failed).into(),
        );
        for (file, err) in result.errors.iter().take(5) {
            web_sys::console::warn_1(&format!("  - {}: {}", file, err).into());
        }
    }

    let loaded = result.loaded as u32;

    // Store in the global
    if CARD_DB.set(db).is_err() {
        return Err(JsError::new("Card database already initialized"));
    }

    web_sys::console::log_1(&format!("[card_loader] Successfully loaded {} cards", loaded).into());

    Ok(loaded)
}

#[wasm_bindgen]
pub fn load_token_bundle(json_str: &str) -> Result<u32, JsError> {
    let bundle: CardBundle = serde_json::from_str(json_str)
        .map_err(|e| JsError::new(&format!("Failed to parse token bundle: {}", e)))?;

    web_sys::console::log_1(
        &format!(
            "[card_loader] Loading {} token scripts from bundle (v{})",
            bundle.cards.len(),
            bundle.version
        )
        .into(),
    );

    let scripts: Vec<(&str, &str)> = bundle
        .cards
        .iter()
        .map(|(filename, content)| (filename.as_str(), content.as_str()))
        .collect();

    let (db, result) = CardDatabase::load_from_strings(scripts);

    if result.failed > 0 {
        web_sys::console::warn_1(
            &format!(
                "[card_loader] {} token scripts failed to parse",
                result.failed
            )
            .into(),
        );
        for (file, err) in result.errors.iter().take(5) {
            web_sys::console::warn_1(&format!("  - {}: {}", file, err).into());
        }
    }

    let loaded = result.loaded as u32;

    if TOKEN_DB.set(db).is_err() {
        return Err(JsError::new("Token database already initialized"));
    }

    web_sys::console::log_1(
        &format!("[card_loader] Successfully loaded {} token scripts", loaded).into(),
    );

    Ok(loaded)
}

/// Check if the card database is loaded.
#[wasm_bindgen]
pub fn is_card_db_loaded() -> bool {
    CARD_DB.get().is_some()
}

/// Get the number of cards in the database.
#[wasm_bindgen]
pub fn get_card_count() -> u32 {
    CARD_DB.get().map(|db| db.len() as u32).unwrap_or(0)
}

/// Get the card database (internal use).
pub fn get_card_db() -> Option<&'static CardDatabase> {
    CARD_DB.get()
}

#[wasm_bindgen]
pub fn is_token_db_loaded() -> bool {
    TOKEN_DB.get().is_some()
}
#[wasm_bindgen]
pub fn get_token_count() -> u32 {
    TOKEN_DB.get().map(|db| db.len() as u32).unwrap_or(0)
}
pub fn get_token_db() -> Option<&'static CardDatabase> {
    TOKEN_DB.get()
}

/// Look up a card by name to verify it exists.
#[wasm_bindgen]
pub fn has_card(name: &str) -> bool {
    CARD_DB
        .get()
        .map(|db| db.get_by_card_name(name).is_some())
        .unwrap_or(false)
}

/// Parse preset decks JSON.
#[wasm_bindgen]
pub fn parse_preset_decks(json_str: &str) -> Result<JsValue, JsError> {
    let decks: Vec<PresetDeck> = serde_json::from_str(json_str)
        .map_err(|e| JsError::new(&format!("Failed to parse preset decks: {}", e)))?;

    serde_wasm_bindgen::to_value(&decks)
        .map_err(|e| JsError::new(&format!("Failed to serialize preset decks: {}", e)))
}
