use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;

use crate::card_db::{card_rules_to_instance, get_card_db};
use rand::SeedableRng;
use tauri::{AppHandle, Emitter, Manager};

use crate::ai_agent::SimpleAiAgent;
use crate::game_view_dto::GameViewDto;
use crate::prompt::{AgentPrompt, PlayerAction};
use crate::tauri_agent::TauriAgent;

pub struct GameManager {
    pub session: Mutex<Option<GameSession>>,
    pub latest_prompt: Arc<Mutex<Option<AgentPrompt>>>,
}

pub struct GameSession {
    pub game_id: String,
    pub response_tx: mpsc::Sender<PlayerAction>,
    pub thread_handle: Option<thread::JoinHandle<()>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            latest_prompt: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_latest_prompt(&self) -> Option<AgentPrompt> {
        self.latest_prompt.lock().ok().and_then(|g| g.clone())
    }

    pub fn start_game(&self, app: AppHandle, deck_list: Vec<String>, starting_life: i32, commander_name: Option<String>) -> Result<String, String> {
        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;

        // End existing session if any
        if let Some(old) = session_guard.take() {
            drop(old.response_tx); // signal game thread to stop (recv returns Err)
            // Don't join — let the old thread wind down while the new game starts
        }
        // Clear any stale prompt from the previous game
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }

        let game_id = format!("game-{}", uuid_simple());
        let game_id_clone = game_id.clone();
        let deck = deck_list;

        // Channels
        let (prompt_tx, prompt_rx) = mpsc::channel::<AgentPrompt>();
        let (response_tx, response_rx) = mpsc::channel::<PlayerAction>();
        let (notify_tx, notify_rx) = mpsc::channel::<String>();

        let response_tx_clone = response_tx.clone();

        // Prompt forwarder thread: reads prompts, stores latest, and emits Tauri events
        let app_prompt = app.clone();
        let latest_prompt = self.latest_prompt.clone();
        thread::spawn(move || {
            eprintln!("[prompt_fwd] Prompt forwarder started");
            let window = app_prompt.get_webview_window("main");
            while let Ok(prompt) = prompt_rx.recv() {
                eprintln!("[prompt_fwd] Got prompt, storing and emitting...");
                // Store latest prompt for polling
                if let Ok(mut lp) = latest_prompt.lock() {
                    *lp = Some(prompt.clone());
                }
                // Emit via both window and app to cover all listener types
                if let Some(ref w) = window {
                    let _ = w.emit("game:prompt", &prompt);
                }
                match app_prompt.emit("game:prompt", &prompt) {
                    Ok(()) => eprintln!("[prompt_fwd] Event emitted OK"),
                    Err(e) => eprintln!("[prompt_fwd] Event emit FAILED: {}", e),
                }
            }
            eprintln!("[prompt_fwd] Prompt forwarder ended (channel closed)");
        });

        // Notify forwarder thread
        let app_notify = app.clone();
        thread::spawn(move || {
            let window = app_notify.get_webview_window("main");
            while let Ok(msg) = notify_rx.recv() {
                let _ = if let Some(ref w) = window {
                    w.emit("game:log", &msg)
                } else {
                    app_notify.emit("game:log", &msg)
                };
            }
        });

        // Game thread
        let handle = thread::spawn(move || {
            eprintln!("[game_thread] Starting game: {} with deck: {:?}", game_id_clone, deck);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_game(game_id_clone.clone(), deck, starting_life, commander_name, prompt_tx, response_rx, notify_tx);
            }));
            match result {
                Ok(()) => eprintln!("[game_thread] Game {} finished normally", game_id_clone),
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "Unknown panic".to_string()
                    };
                    eprintln!("[game_thread] PANIC in game {}: {}", game_id_clone, msg);
                }
            }
        });

        *session_guard = Some(GameSession {
            game_id: game_id.clone(),
            response_tx: response_tx_clone,
            thread_handle: Some(handle),
        });

        Ok(game_id)
    }

    pub fn respond(&self, app: AppHandle, action: PlayerAction) -> Result<(), String> {
        if matches!(action, PlayerAction::Concede) {
            // Build a synthetic game-over prompt using the last known game view
            let game_view = {
                let lp = self.latest_prompt.lock().map_err(|e| e.to_string())?;
                let base_view = lp.as_ref().and_then(|p| match p {
                    AgentPrompt::ChooseAction { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseAttackers { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseBlockers { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetPlayer { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetCard { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetAny { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::Mulligan { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::GameOver { game_view } => Some(game_view.clone()),
                });
                let mut view = base_view.unwrap_or_else(|| GameViewDto {
                    game_id: String::new(),
                    turn: 0,
                    step: "main1".into(),
                    active_player_id: String::new(),
                    priority_player_id: String::new(),
                    players: vec![],
                    my_hand: vec![],
                    battlefield: vec![],
                    stack: vec![],
                    exile: vec![],
                    graveyard: vec![],
                    opponent_graveyard: vec![],
                    opponent_exile: vec![],
                    my_command_zone: vec![],
                    opponent_command_zone: vec![],
                    game_over: false,
                    winner_id: None,
                });
                let opponent_id = view.players.iter().find(|p| !p.is_human).map(|p| p.id.clone());
                view.game_over = true;
                view.winner_id = opponent_id;
                view
            };
            let prompt = AgentPrompt::GameOver { game_view };
            if let Ok(mut lp) = self.latest_prompt.lock() {
                *lp = Some(prompt.clone());
            }
            let _ = app.emit("game:prompt", &prompt);
            // Unblock the game thread with a no-op
            let session_guard = self.session.lock().map_err(|e| e.to_string())?;
            if let Some(session) = session_guard.as_ref() {
                let _ = session.response_tx.send(PlayerAction::PlayCard { card_id: None });
            }
            return Ok(());
        }

        let session_guard = self.session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = session_guard.as_ref() {
            session
                .response_tx
                .send(action)
                .map_err(|e| format!("Game thread not responding: {}", e))?;
            Ok(())
        } else {
            Err("No active game session".into())
        }
    }

    pub fn end_game(&self) -> Result<(), String> {
        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = session_guard.take() {
            drop(session.response_tx); // signals game thread to stop
            // Don't join here — let the thread wind down on its own so end_game returns fast
        }
        // Clear latest prompt so the next game doesn't see stale state
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }
        Ok(())
    }
}

fn uuid_simple() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:08x}{:08x}", rng.gen::<u32>(), rng.gen::<u32>())
}

fn run_game(
    game_id: String,
    deck_list: Vec<String>,
    starting_life: i32,
    commander_name: Option<String>,
    prompt_tx: mpsc::Sender<AgentPrompt>,
    response_rx: mpsc::Receiver<PlayerAction>,
    notify_tx: mpsc::Sender<String>,
) {
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let mut game = GameState::new(&["You", "AI Opponent"], starting_life);

    // Build human player deck: if a single preset ID is given, use that;
    // otherwise build a custom deck from the card name list.
    let is_preset = deck_list.len() == 1 && matches!(
        deck_list[0].as_str(),
        "red_burn" | "green_stompy" | "white_aggro" | "black_control"
    );

    if is_preset {
        match deck_list[0].as_str() {
            "green_stompy" => {
                build_named_deck(&mut game, p0, GREEN_STOMPY);
                build_named_deck(&mut game, p1, RED_BURN);
            }
            "white_aggro" => {
                build_named_deck(&mut game, p0, WHITE_AGGRO);
                build_named_deck(&mut game, p1, BLACK_CONTROL);
            }
            "black_control" => {
                build_named_deck(&mut game, p0, BLACK_CONTROL);
                build_named_deck(&mut game, p1, WHITE_AGGRO);
            }
            _ => {
                // red_burn (default)
                build_named_deck(&mut game, p0, RED_BURN);
                build_named_deck(&mut game, p1, GREEN_STOMPY);
            }
        }
    } else {
        // Custom deck: build human player deck from card names sent by the frontend.
        build_custom_deck(&mut game, p0, &deck_list);
        // AI always plays red burn as a simple opponent.
        build_named_deck(&mut game, p1, RED_BURN);
    }

    // Designate commander for the human player (must happen before game_loop.run which shuffles).
    if let Some(ref name) = commander_name {
        let library_cards: Vec<CardId> = game.cards_in_zone(ZoneType::Library, p0).to_vec();
        for cid in library_cards {
            if game.card(cid).card_name == *name {
                game.card_mut(cid).is_commander = true;
                game.move_card(cid, ZoneType::Command, p0);
                eprintln!("[game] Designated '{}' as commander for player 0", name);
                break;
            }
        }
    }

    let human = TauriAgent::new(p0, game_id.clone(), prompt_tx.clone(), response_rx, notify_tx);
    let ai = SimpleAiAgent;

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];
    let mut game_loop = GameLoop::new(2);
    let mut rng = rand::rngs::StdRng::from_entropy();

    let winner = game_loop.run(&mut game, &mut agents, &mut rng, 50);

    // Send final game-over prompt
    let final_view = GameViewDto::from_engine(&game, &game_loop.mana_pools, p0, &game_id, &[], &[]);
    let _ = prompt_tx.send(AgentPrompt::GameOver {
        game_view: final_view,
    });

    let _ = winner; // winner is also in the game_view
}

// ── Preset deck lists ──────────────────────────────────────────────
//
// Each entry is (card_name, count). Card definitions come exclusively from
// the Forge card scripts in forge/forge-gui/res/cardsfolder/ — no stats are
// hardcoded here.

const RED_BURN: &[(&str, usize)] = &[
    ("Mountain", 17),
    ("Lightning Bolt", 4),
    ("Shock", 4),
    ("Gray Ogre", 3),
    ("Hill Giant", 3),
    ("Guttersnipe", 3),
];

const GREEN_STOMPY: &[(&str, usize)] = &[
    ("Forest", 17),
    ("Giant Growth", 4),
    ("Grizzly Bears", 3),
    ("Centaur Courser", 2),
    ("Garruk's Companion", 3),
    ("Giant Spider", 2),
    ("Wall of Ice", 2),
    ("Craw Wurm", 2),
];

const WHITE_AGGRO: &[(&str, usize)] = &[
    ("Plains", 17),
    ("Savannah Lions", 4),
    ("White Knight", 3),
    ("Serra Angel", 3),
    ("Soul Warden", 3),
];

const BLACK_CONTROL: &[(&str, usize)] = &[
    ("Swamp", 13),
    ("Island", 4),
    ("Doom Blade", 4),
    ("Divination", 2),
    ("Typhoid Rats", 3),
    ("Vampire Nighthawk", 3),
    ("Mulldrifter", 2),
];

// ── Deck builders ──────────────────────────────────────────────────

/// Build a preset deck from a (name, count) list, loading each card definition
/// from the global CardDatabase (parsed from the Forge card scripts).
fn build_named_deck(game: &mut GameState, owner: PlayerId, deck: &[(&str, usize)]) {
    let db = get_card_db();
    for (name, count) in deck {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let card = card_rules_to_instance(rules, owner);
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => eprintln!("[deck] Unknown card '{}' — skipped", name),
        }
    }
}

/// Build a custom deck for `owner` from a list of card names (one name per
/// copy), loading each definition from the global CardDatabase.
/// Unrecognised names are skipped with a log message.
fn build_custom_deck(game: &mut GameState, owner: PlayerId, names: &[String]) {
    let db = get_card_db();
    for name in names {
        match db.get_by_card_name(name) {
            Some(rules) => {
                let card = card_rules_to_instance(rules, owner);
                let id = game.create_card(card);
                game.move_card(id, ZoneType::Library, owner);
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", name),
        }
    }
}
