//! Main WASM API for the game engine.
//!
//! This module provides the JavaScript-facing API for the forge-engine.

use forge_agent_interface::deck_dto::Deck as WireDeck;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::card_loader::{get_card_db, DeckCard};
use crate::game_runner::{GameConfig as RustGameConfig, WasmGame};

/// Flatten every playable pile of a wire `Deck` (main + sideboard +
/// commanders + supplementary decks) into the internal `DeckCard`
/// shape the engine consumes.
fn deck_cards_for_engine(deck: &WireDeck) -> Vec<DeckCard> {
    let mut out: Vec<DeckCard> = Vec::with_capacity(deck.cards.len());
    let push = |out: &mut Vec<DeckCard>, list: &[forge_agent_interface::deck_dto::DeckCard]| {
        for c in list {
            out.push(DeckCard {
                name: c.identity.name.clone(),
                count: 1,
                set_code: c.identity.set_code.clone(),
                card_number: c.identity.card_number.clone(),
            });
        }
    };
    push(&mut out, &deck.cards);
    push(&mut out, &deck.sideboard);
    if let Some(commanders) = &deck.commanders {
        push(&mut out, commanders);
    }
    if let Some(attractions) = &deck.attractions {
        push(&mut out, attractions);
    }
    if let Some(contraptions) = &deck.contraptions {
        push(&mut out, contraptions);
    }
    if let Some(schemes) = &deck.schemes {
        push(&mut out, schemes);
    }
    if let Some(planes) = &deck.planes {
        push(&mut out, planes);
    }
    out
}

/// Game configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    #[serde(default = "default_starting_life")]
    pub starting_life: i32,
    #[serde(default)]
    pub commander_name: Option<String>,
}

fn default_starting_life() -> i32 {
    20
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            starting_life: 20,
            commander_name: None,
        }
    }
}

/// Engine information for version checking.
#[derive(Debug, Clone, Serialize)]
pub struct EngineInfo {
    pub version: String,
    pub wasm_ready: bool,
}

/// Get engine information.
#[wasm_bindgen]
pub fn get_engine_info() -> JsValue {
    let info = EngineInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        wasm_ready: true,
    };
    serde_wasm_bindgen::to_value(&info).unwrap_or(JsValue::NULL)
}

/// Verify WASM is working by echoing back a message.
#[wasm_bindgen]
pub fn echo(msg: &str) -> String {
    format!("forge-wasm echo: {}", msg)
}

/// Parse a deck from JSON.
///
/// Returns a summary of the parsed deck for verification.
#[wasm_bindgen]
pub fn parse_deck(deck_json: JsValue) -> Result<JsValue, JsError> {
    let deck: WireDeck = serde_wasm_bindgen::from_value(deck_json)
        .map_err(|e| JsError::new(&format!("Failed to parse deck: {}", e)))?;

    #[derive(Serialize)]
    struct DeckSummary {
        card_count: usize,
        card_names: Vec<String>,
    }

    let summary = DeckSummary {
        card_count: deck.cards.len(),
        card_names: deck.cards.iter().map(|c| c.identity.name.clone()).collect(),
    };

    serde_wasm_bindgen::to_value(&summary)
        .map_err(|e| JsError::new(&format!("Failed to serialize summary: {}", e)))
}

/// Parse a game config from JSON.
#[wasm_bindgen]
pub fn parse_config(config_json: JsValue) -> Result<JsValue, JsError> {
    let config: GameConfig = if config_json.is_undefined() || config_json.is_null() {
        GameConfig::default()
    } else {
        serde_wasm_bindgen::from_value(config_json)
            .map_err(|e| JsError::new(&format!("Failed to parse config: {}", e)))?
    };

    serde_wasm_bindgen::to_value(&config)
        .map_err(|e| JsError::new(&format!("Failed to serialize config: {}", e)))
}

/// Test that the RNG works in WASM.
#[wasm_bindgen]
pub fn test_rng() -> JsValue {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let values: Vec<u32> = (0..5).map(|_| rng.gen_range(0..100)).collect();

    serde_wasm_bindgen::to_value(&values).unwrap_or(JsValue::NULL)
}

/// Test that forge-foundation types work.
#[wasm_bindgen]
pub fn test_foundation() -> JsValue {
    use forge_foundation::{Color, PhaseType, ZoneType};

    #[derive(Serialize)]
    struct FoundationTest {
        colors: Vec<String>,
        phases: Vec<String>,
        zones: Vec<String>,
    }

    let test = FoundationTest {
        colors: vec![
            format!("{:?}", Color::White),
            format!("{:?}", Color::Blue),
            format!("{:?}", Color::Black),
            format!("{:?}", Color::Red),
            format!("{:?}", Color::Green),
        ],
        phases: vec![
            format!("{:?}", PhaseType::Untap),
            format!("{:?}", PhaseType::Main1),
            format!("{:?}", PhaseType::CombatBegin),
            format!("{:?}", PhaseType::Main2),
        ],
        zones: vec![
            format!("{:?}", ZoneType::Hand),
            format!("{:?}", ZoneType::Library),
            format!("{:?}", ZoneType::Battlefield),
            format!("{:?}", ZoneType::Graveyard),
        ],
    };

    serde_wasm_bindgen::to_value(&test).unwrap_or(JsValue::NULL)
}

// ============================================================================
// Full Game API
// ============================================================================

// ============================================================================
// Interactive Game API (uses shared PromptAgent + Atomics.wait)
// ============================================================================

use crate::wasm_transport::{WasmAiTransport, WasmTransport};
use forge_agent_interface::agent_impl::PromptAgent;

/// Run an interactive game with a human player (blocking on Atomics.wait).
///
/// This function blocks the worker thread until the game is complete.
/// The human player's prompts are written to the SharedArrayBuffer,
/// and the worker blocks until the main thread provides a response.
///
/// Call this from a Web Worker — it will block the thread.
#[wasm_bindgen]
pub fn run_interactive_game(
    human_deck_json: JsValue,
    ai_deck_json: JsValue,
    config_json: JsValue,
    shared_buffer: JsValue,
) -> Result<JsValue, JsError> {
    use forge_engine_core::agent::PlayerAgent;
    use forge_engine_core::ids::PlayerId;
    use js_sys::SharedArrayBuffer;

    // Check card database
    if get_card_db().is_none() {
        return Err(JsError::new("Card database not loaded"));
    }

    // Parse decks
    let human_deck: WireDeck = serde_wasm_bindgen::from_value(human_deck_json)
        .map_err(|e| JsError::new(&format!("Failed to parse human deck: {}", e)))?;
    let ai_deck: WireDeck = serde_wasm_bindgen::from_value(ai_deck_json)
        .map_err(|e| JsError::new(&format!("Failed to parse AI deck: {}", e)))?;

    // Parse config
    let config: RustGameConfig = if config_json.is_undefined() || config_json.is_null() {
        RustGameConfig::default()
    } else {
        serde_wasm_bindgen::from_value(config_json)
            .map_err(|e| JsError::new(&format!("Failed to parse config: {}", e)))?
    };

    // Convert decks
    let human_cards: Vec<DeckCard> = deck_cards_for_engine(&human_deck);
    let ai_cards: Vec<DeckCard> = deck_cards_for_engine(&ai_deck);

    web_sys::console::log_1(
        &format!(
            "[InteractiveGame] Starting game: {} human cards vs {} AI cards",
            human_cards.len(),
            ai_cards.len()
        )
        .into(),
    );

    // Create the game
    let mut wasm_game = WasmGame::new(&human_cards, &ai_cards, &config)
        .map_err(|e| JsError::new(&format!("Failed to create game: {}", e)))?;

    // Create the SharedArrayBuffer-backed transport for human player
    let sab: SharedArrayBuffer = shared_buffer
        .dyn_into()
        .map_err(|_| JsError::new("Expected SharedArrayBuffer"))?;

    let human_transport = WasmTransport::new(&sab);
    let ai_transport = WasmAiTransport;

    let game_id = format!("wasm-interactive-{}", js_sys::Date::now() as u64);

    // Create agents using the shared crate's PromptAgent
    let human_agent = PromptAgent::new(PlayerId(0), game_id.clone(), human_transport);
    let ai_agent = PromptAgent::new(PlayerId(1), game_id.clone(), ai_transport);

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human_agent), Box::new(ai_agent)];

    web_sys::console::log_1(&"[InteractiveGame] Agents created, starting game loop".into());

    // Run the game loop — this BLOCKS on Atomics.wait() when human input is needed
    let winner = wasm_game.game_loop.run(
        &mut wasm_game.game_state,
        &mut agents,
        &mut wasm_game.rng,
        5000, // max turns
    );

    web_sys::console::log_1(
        &format!("[InteractiveGame] Game complete. Winner: {:?}", winner).into(),
    );

    // Return final result
    #[derive(Serialize)]
    struct InteractiveGameResult {
        winner_id: Option<u32>,
        game_over: bool,
    }

    let result = InteractiveGameResult {
        winner_id: winner.map(|p| p.0),
        game_over: true,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize result: {}", e)))
}

/// Run a multiplayer game with two players using separate SharedArrayBuffers.
///
/// Player 0 (local) uses `local_buffer` — prompts shown in UI.
/// Player 1 (remote) uses `remote_buffer` — prompts relayed via WebSocket.
/// Both block on Atomics.wait() sequentially (never concurrently).
#[wasm_bindgen]
pub fn run_multiplayer_game(
    player0_deck_json: JsValue,
    player1_deck_json: JsValue,
    config_json: JsValue,
    local_buffer: JsValue,
    remote_buffer: JsValue,
    local_player_index: u32,
) -> Result<JsValue, JsError> {
    use forge_engine_core::agent::PlayerAgent;
    use forge_engine_core::ids::PlayerId;
    use js_sys::SharedArrayBuffer;

    if get_card_db().is_none() {
        return Err(JsError::new("Card database not loaded"));
    }

    let deck0: WireDeck = serde_wasm_bindgen::from_value(player0_deck_json)
        .map_err(|e| JsError::new(&format!("Failed to parse player 0 deck: {}", e)))?;
    let deck1: WireDeck = serde_wasm_bindgen::from_value(player1_deck_json)
        .map_err(|e| JsError::new(&format!("Failed to parse player 1 deck: {}", e)))?;

    let config: RustGameConfig = if config_json.is_undefined() || config_json.is_null() {
        RustGameConfig::default()
    } else {
        serde_wasm_bindgen::from_value(config_json)
            .map_err(|e| JsError::new(&format!("Failed to parse config: {}", e)))?
    };

    let cards0: Vec<DeckCard> = deck_cards_for_engine(&deck0);
    let cards1: Vec<DeckCard> = deck_cards_for_engine(&deck1);

    let mut wasm_game = WasmGame::new(&cards0, &cards1, &config)
        .map_err(|e| JsError::new(&format!("Failed to create game: {}", e)))?;

    let local_sab: SharedArrayBuffer = local_buffer
        .dyn_into()
        .map_err(|_| JsError::new("Expected SharedArrayBuffer for local player"))?;
    let remote_sab: SharedArrayBuffer = remote_buffer
        .dyn_into()
        .map_err(|_| JsError::new("Expected SharedArrayBuffer for remote player"))?;

    let game_id = format!("wasm-mp-{}", js_sys::Date::now() as u64);

    // Create agents — both use WasmTransport with separate SABs
    let (agent0, agent1): (Box<dyn PlayerAgent>, Box<dyn PlayerAgent>) = if local_player_index == 0
    {
        (
            Box::new(PromptAgent::new(
                PlayerId(0),
                game_id.clone(),
                WasmTransport::new(&local_sab),
            )),
            Box::new(PromptAgent::new(
                PlayerId(1),
                game_id.clone(),
                WasmTransport::new(&remote_sab),
            )),
        )
    } else {
        (
            Box::new(PromptAgent::new(
                PlayerId(0),
                game_id.clone(),
                WasmTransport::new(&remote_sab),
            )),
            Box::new(PromptAgent::new(
                PlayerId(1),
                game_id.clone(),
                WasmTransport::new(&local_sab),
            )),
        )
    };

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![agent0, agent1];

    web_sys::console::log_1(
        &format!(
            "[MultiplayerGame] Starting: local=player-{}, {} vs {} cards",
            local_player_index,
            cards0.len(),
            cards1.len()
        )
        .into(),
    );

    let winner = wasm_game.game_loop.run(
        &mut wasm_game.game_state,
        &mut agents,
        &mut wasm_game.rng,
        5000,
    );

    web_sys::console::log_1(&format!("[MultiplayerGame] Complete. Winner: {:?}", winner).into());

    #[derive(Serialize)]
    struct InteractiveGameResult {
        winner_id: Option<u32>,
        game_over: bool,
    }

    serde_wasm_bindgen::to_value(&InteractiveGameResult {
        winner_id: winner.map(|p| p.0),
        game_over: true,
    })
    .map_err(|e| JsError::new(&format!("Failed to serialize result: {}", e)))
}
