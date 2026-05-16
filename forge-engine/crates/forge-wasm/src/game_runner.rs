//! Game runner for WASM.
//!
//! This module handles running actual games in the browser.

use forge_carddb::CardRules;
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::PlayerId;
use forge_engine_core::player::RegisteredPlayer;
use forge_foundation::ZoneType;
use rand::SeedableRng;
use serde::Deserialize;

use crate::card_loader::{get_card_db, get_token_db, DeckCard};

/// A prepared player with cards ready to be instantiated.
pub struct PreparedPlayer {
    pub registered: RegisteredPlayer,
    pub cards: Vec<(CardInstance, ZoneType)>,
}

/// Game configuration from JavaScript.
#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    #[serde(default = "default_starting_life")]
    pub starting_life: i32,
    #[serde(default)]
    #[allow(dead_code)]
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

/// Convert CardRules to CardInstance.
pub fn card_rules_to_instance(rules: &CardRules, owner: PlayerId) -> CardInstance {
    CardInstance::from_rules(rules, owner)
}

/// Prepare a player from a deck list.
pub fn prepare_player(
    name: &str,
    deck_cards: &[DeckCard],
    starting_life: i32,
) -> Result<PreparedPlayer, String> {
    let card_db = get_card_db().ok_or("Card database not loaded")?;

    let mut deck_names: Vec<String> = Vec::new();
    let mut cards: Vec<(CardInstance, ZoneType)> = Vec::new();

    let mut skipped: Vec<String> = Vec::new();
    for deck_card in deck_cards {
        let Some(rules) = card_db.get_by_card_name(&deck_card.name) else {
            skipped.push(deck_card.name.clone());
            continue;
        };
        for _ in 0..deck_card.count {
            deck_names.push(rules.name());
            let mut instance = card_rules_to_instance(rules, PlayerId(0)); // Owner set later
            if !deck_card.set_code.is_empty() {
                instance.set_code = Some(deck_card.set_code.clone());
            }
            if !deck_card.card_number.is_empty() {
                instance.card_number = Some(deck_card.card_number.clone());
            }
            cards.push((instance, ZoneType::Library));
        }
    }
    if !skipped.is_empty() {
        web_sys::console::warn_1(
            &format!(
                "[deck] Unknown card(s) skipped for {name}: {}",
                skipped.join(", ")
            )
            .into(),
        );
    }

    let mut registered = RegisteredPlayer::new(name);
    registered.starting_life = starting_life;
    registered.current_deck = deck_names.clone();
    registered.original_deck = deck_names;

    Ok(PreparedPlayer { registered, cards })
}

/// Prepare an AI player with a simple deck.
pub fn prepare_ai_player(
    name: &str,
    deck_cards: &[DeckCard],
    starting_life: i32,
) -> Result<PreparedPlayer, String> {
    prepare_player(name, deck_cards, starting_life)
}

/// Game state wrapper for WASM.
pub struct WasmGame {
    pub game_state: GameState,
    pub game_loop: GameLoop,
    pub rng: rand::rngs::StdRng,
    #[allow(dead_code)]
    pub human_player_id: PlayerId,
    #[allow(dead_code)]
    pub ai_player_id: PlayerId,
}

impl WasmGame {
    /// Create a new game with the given players.
    pub fn new(
        human_deck: &[DeckCard],
        ai_deck: &[DeckCard],
        config: &GameConfig,
    ) -> Result<Self, String> {
        let starting_life = config.starting_life;

        // Prepare players
        let human = prepare_player("You", human_deck, starting_life)?;
        let ai = prepare_ai_player("AI Opponent", ai_deck, starting_life)?;

        let registered: Vec<RegisteredPlayer> =
            vec![human.registered.clone(), ai.registered.clone()];

        // Create game state
        let mut game_state = GameState::new_from_registered_players(&registered);

        // Set player owners correctly and add cards
        let human_pid = PlayerId(0);
        let ai_pid = PlayerId(1);

        let human_cards: Vec<(CardInstance, ZoneType)> = human
            .cards
            .into_iter()
            .map(|(mut card, zone)| {
                card.owner = human_pid;
                card.controller = human_pid;
                (card, zone)
            })
            .collect();

        let ai_cards: Vec<(CardInstance, ZoneType)> = ai
            .cards
            .into_iter()
            .map(|(mut card, zone)| {
                card.owner = ai_pid;
                card.controller = ai_pid;
                (card, zone)
            })
            .collect();

        game_state.initialize_registered_player_cards(
            human_pid,
            &human.registered,
            human_cards,
            None,
        );
        game_state.initialize_registered_player_cards(ai_pid, &ai.registered, ai_cards, None);

        // Create game loop
        let mut game_loop = GameLoop::new(2);
        if let Some(token_db) = get_token_db() {
            for (script_name, rules) in token_db.iter() {
                let template = card_rules_to_instance(rules, human_pid);
                game_loop.register_token(script_name.clone(), template);
            }
        }

        // Create RNG
        let rng = rand::rngs::StdRng::from_entropy();

        Ok(WasmGame {
            game_state,
            game_loop,
            rng,
            human_player_id: human_pid,
            ai_player_id: ai_pid,
        })
    }
}
