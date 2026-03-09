use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;

use rand::SeedableRng;
use tauri::{AppHandle, Emitter};

use crate::ai_agent::spawn_ai_prompt_responder;
use crate::card_db::{card_rules_to_instance, get_token_db};
use crate::game_log_event::GameLogEntryDto;
use crate::game_snapshot_event::GameSnapshotEventDto;
use crate::game_view_dto::GameViewDto;
use crate::ids_codec::player_slot;
use crate::multiplayer_controller::{
    parse_remote_response, spawn_engine_prompt_forwarder, spawn_notify_forwarder,
    spawn_remote_prompt_forwarder, spawn_snapshot_forwarder,
};
use crate::preset_decks::{
    build_ai_opponent, build_custom_deck, build_preset_deck_for_player, build_preset_decks,
    is_preset_id, CardIdentity,
};
use crate::prompt::{AgentPrompt, AgentPromptInner, PlayerAction};
use crate::tauri_agent::TauriAgent;

pub struct GameManager {
    pub session: Mutex<Option<GameSession>>,
    pub latest_prompt: Arc<Mutex<Option<AgentPrompt>>>,
}

pub struct GameSession {
    pub game_id: String,
    pub response_tx: mpsc::Sender<PlayerAction>,
    /// Per-remote-player response channels (player_index → sender).
    pub remote_response_txs: HashMap<usize, mpsc::Sender<PlayerAction>>,
    pub thread_handle: Option<thread::JoinHandle<()>>,
    pub is_multiplayer: bool,
    pub is_host: bool,
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

    pub fn start_game(
        &self,
        app: AppHandle,
        deck_list: Vec<CardIdentity>,
        starting_life: i32,
        commander_name: Option<String>,
    ) -> Result<String, String> {
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
        let (notify_tx, notify_rx) = mpsc::channel::<GameLogEntryDto>();
        let (snapshot_tx, snapshot_rx) = mpsc::channel::<GameSnapshotEventDto>();

        let response_tx_clone = response_tx.clone();

        spawn_engine_prompt_forwarder(app.clone(), self.latest_prompt.clone(), prompt_rx);
        spawn_notify_forwarder(app.clone(), notify_rx, None);
        spawn_snapshot_forwarder(app.clone(), snapshot_rx, None);

        // Game thread
        let handle = thread::spawn(move || {
            eprintln!(
                "[game_thread] Starting game: {} with deck: {:?}",
                game_id_clone, deck
            );
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_game(
                    game_id_clone.clone(),
                    deck,
                    starting_life,
                    commander_name,
                    prompt_tx,
                    response_rx,
                    notify_tx,
                    snapshot_tx,
                );
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
            remote_response_txs: HashMap::new(),
            thread_handle: Some(handle),
            is_multiplayer: false,
            is_host: true,
        });

        Ok(game_id)
    }

    pub fn respond(&self, app: AppHandle, action: PlayerAction) -> Result<(), String> {
        if matches!(action, PlayerAction::Concede) {
            // Build a synthetic game-over prompt using the last known game view
            let game_view = {
                let lp = self.latest_prompt.lock().map_err(|e| e.to_string())?;
                let base_view = lp.as_ref().map(|p| p.inner.game_view().clone());
                let mut view = base_view.unwrap_or_else(|| GameViewDto::empty(String::new()));
                let opponent_id = view
                    .players
                    .iter()
                    .find(|p| !p.is_human)
                    .map(|p| p.id.clone());
                view.game_over = true;
                view.winner_id = opponent_id;
                view
            };
            let prompt = AgentPrompt {
                display_events: vec![],
                inner: AgentPromptInner::GameOver { game_view },
            };
            if let Ok(mut lp) = self.latest_prompt.lock() {
                *lp = Some(prompt.clone());
            }
            let _ = app.emit("game:prompt", &prompt);
            // Unblock the game thread with a no-op
            let session_guard = self.session.lock().map_err(|e| e.to_string())?;
            if let Some(session) = session_guard.as_ref() {
                let _ = session
                    .response_tx
                    .send(PlayerAction::PlayCard { card_id: None, mode: None });
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
            drop(session.remote_response_txs); // drop remote channels too
                                               // Don't join here — let the thread wind down on its own so end_game returns fast
        }
        // Clear latest prompt so the next game doesn't see stale state
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }
        Ok(())
    }

    pub fn start_multiplayer_game(
        &self,
        app: AppHandle,
        player_names: Vec<String>,
        deck_lists: Vec<Vec<CardIdentity>>,
        engine_player_index: usize,
        local_is_host: bool,
        starting_life: i32,
    ) -> Result<String, String> {
        let num_players = player_names.len();
        if num_players < 2 {
            return Err("Need at least 2 players".into());
        }
        if engine_player_index >= num_players {
            return Err("Invalid engine player index".into());
        }
        if deck_lists.len() != num_players {
            return Err("Deck list count must match player count".into());
        }
        if deck_lists.iter().any(|deck| deck.is_empty()) {
            return Err("All players must have a selected deck".into());
        }

        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;

        // End existing session if any
        if let Some(old) = session_guard.take() {
            drop(old.response_tx);
            drop(old.remote_response_txs);
        }
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }

        // Multiplayer is single-engine authoritative; non-host peers run relay-only.
        // Host identity is provided by the client from lobby state.
        if !local_is_host {
            return Ok("relay-only".into());
        }

        let game_id = format!("game-{}", uuid_simple());
        let game_id_clone = game_id.clone();

        // Engine-local player channels (TauriAgent)
        let (engine_prompt_tx, engine_prompt_rx) = mpsc::channel::<AgentPrompt>();
        let (engine_response_tx, engine_response_rx) = mpsc::channel::<PlayerAction>();
        let (engine_notify_tx, notify_rx) = mpsc::channel::<GameLogEntryDto>();
        let (engine_snapshot_tx, snapshot_rx) = mpsc::channel::<GameSnapshotEventDto>();

        let engine_response_tx_clone = engine_response_tx.clone();

        // Remote players: shared prompt channel and per-player response channels
        let (remote_prompt_tx, remote_prompt_rx) = mpsc::channel::<(usize, AgentPrompt)>();
        let mut remote_response_txs: HashMap<usize, mpsc::Sender<PlayerAction>> = HashMap::new();
        let mut remote_response_rxs: Vec<(usize, mpsc::Receiver<PlayerAction>)> = Vec::new();

        for i in 0..num_players {
            if i != engine_player_index {
                let (resp_tx, resp_rx) = mpsc::channel::<PlayerAction>();
                remote_response_txs.insert(i, resp_tx);
                remote_response_rxs.push((i, resp_rx));
            }
        }

        // Keep clones for the game thread (which builds agents internally)
        let game_engine_prompt_tx = engine_prompt_tx.clone();
        let game_remote_prompt_tx = remote_prompt_tx.clone();

        // Drop extra senders so forwarders terminate when the game thread ends
        drop(engine_prompt_tx);
        drop(remote_prompt_tx);

        spawn_engine_prompt_forwarder(app.clone(), self.latest_prompt.clone(), engine_prompt_rx);
        spawn_notify_forwarder(
            app.clone(),
            notify_rx,
            Some(player_slot(engine_player_index)),
        );
        spawn_snapshot_forwarder(
            app.clone(),
            snapshot_rx,
            Some(player_slot(engine_player_index)),
        );
        spawn_remote_prompt_forwarder(app.clone(), remote_prompt_rx);

        // Game thread
        let player_name_strs = player_names.clone();
        let selected_deck_lists = deck_lists.clone();
        let handle = thread::spawn(move || {
            eprintln!(
                "[game_thread] Starting multiplayer game: {} with {} players",
                game_id_clone, num_players
            );
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_multiplayer_game(
                    game_id_clone.clone(),
                    player_name_strs,
                    selected_deck_lists,
                    engine_player_index,
                    starting_life,
                    game_engine_prompt_tx,
                    engine_response_rx,
                    engine_notify_tx,
                    engine_snapshot_tx,
                    game_remote_prompt_tx,
                    remote_response_rxs,
                );
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
            response_tx: engine_response_tx_clone,
            remote_response_txs,
            thread_handle: Some(handle),
            is_multiplayer: true,
            is_host: local_is_host,
        });

        Ok(game_id)
    }

    /// Route a response from a remote player to the appropriate agent's channel.
    pub fn route_remote_response(&self, state: &serde_json::Value) {
        let (player_index, action) = match parse_remote_response(state) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[route] {}", e);
                return;
            }
        };

        let session_guard = match self.session.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(session) = session_guard.as_ref() {
            if let Some(tx) = session.remote_response_txs.get(&player_index) {
                if let Err(e) = tx.send(action) {
                    eprintln!("[route] Failed to send to player {}: {}", player_index, e);
                }
            } else {
                eprintln!("[route] No channel for player index {}", player_index);
            }
        }
    }

    pub fn restore_snapshot(&self, checkpoint_id: u64) -> Result<(), String> {
        let session_guard = self.session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = session_guard.as_ref() {
            if session.is_multiplayer && !session.is_host {
                return Err("Only the host can restore snapshots".into());
            }
            session
                .response_tx
                .send(PlayerAction::RestoreSnapshot { checkpoint_id })
                .map_err(|e| format!("Game thread not responding: {}", e))?;
            Ok(())
        } else {
            Err("No active game session".into())
        }
    }
}

fn uuid_simple() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:08x}{:08x}", rng.gen::<u32>(), rng.gen::<u32>())
}

fn run_game(
    game_id: String,
    deck_list: Vec<CardIdentity>,
    starting_life: i32,
    commander_name: Option<String>,
    prompt_tx: mpsc::Sender<AgentPrompt>,
    response_rx: mpsc::Receiver<PlayerAction>,
    notify_tx: mpsc::Sender<GameLogEntryDto>,
    snapshot_tx: mpsc::Sender<GameSnapshotEventDto>,
) {
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let mut game = GameState::new(&["You", "AI Opponent"], starting_life);

    // Build human player deck: if a single preset ID is given, use that;
    // otherwise build a custom deck from the card name list.
    if deck_list.len() == 1 && is_preset_id(&deck_list[0].name) {
        build_preset_decks(&mut game, &deck_list[0].name, p0, p1);
    } else {
        // Custom deck: build human player deck from card names sent by the frontend.
        build_custom_deck(&mut game, p0, &deck_list);
        // AI always plays red burn as a simple opponent.
        build_ai_opponent(&mut game, p1);
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

    let human = TauriAgent::new_local(
        p0,
        game_id.clone(),
        prompt_tx.clone(),
        response_rx,
        notify_tx,
        snapshot_tx,
    );

    let (ai_prompt_tx, ai_prompt_rx) = mpsc::channel::<AgentPrompt>();
    let (ai_response_tx, ai_response_rx) = mpsc::channel::<PlayerAction>();
    spawn_ai_prompt_responder(ai_prompt_rx, ai_response_tx);
    let ai = TauriAgent::new_ai(p1, game_id.clone(), ai_prompt_tx, ai_response_rx);

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];
    let mut game_loop = GameLoop::new(2);
    if std::env::var("FORGE_ENGINE_GAME_LOG").is_err() {
        game_loop.game_log.set_enabled(true);
    }
    game_loop.experimental_restore_snapshot =
        std::env::var("FORGE_ENGINE_RESTORE_SNAPSHOT").is_ok();

    // Register token templates so the engine can instantiate tokens by script name.
    // Uses a placeholder owner (p0); the actual owner/controller is set at creation time.
    let token_db = get_token_db();
    for (script_name, rules) in token_db.iter() {
        let template = card_rules_to_instance(rules, p0);
        game_loop.register_token(script_name.clone(), template);
    }

    let mut rng = rand::rngs::StdRng::from_entropy();

    let winner = game_loop.run(&mut game, &mut agents, &mut rng, 5000);

    // Send final game-over prompt
    let final_view = GameViewDto::from_engine(&game, &game_loop.mana_pools, p0, &game_id, &[], &[]);
    let _ = prompt_tx.send(AgentPrompt {
        display_events: vec![],
        inner: AgentPromptInner::GameOver {
            game_view: final_view,
        },
    });

    let _ = winner; // winner is also in the game_view
}

fn run_multiplayer_game(
    game_id: String,
    player_names: Vec<String>,
    deck_lists: Vec<Vec<CardIdentity>>,
    engine_player_index: usize,
    starting_life: i32,
    engine_prompt_tx: mpsc::Sender<AgentPrompt>,
    engine_response_rx: mpsc::Receiver<PlayerAction>,
    engine_notify_tx: mpsc::Sender<GameLogEntryDto>,
    engine_snapshot_tx: mpsc::Sender<GameSnapshotEventDto>,
    remote_prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
    remote_response_rxs: Vec<(usize, mpsc::Receiver<PlayerAction>)>,
) {
    let num_players = player_names.len();
    let name_refs: Vec<&str> = player_names.iter().map(|s| s.as_str()).collect();
    let mut game = GameState::new(&name_refs, starting_life);

    // Build agents inside the thread (avoids Send issues with trait objects).
    let engine_pid = PlayerId(engine_player_index as u32);
    let mut engine_agent: Option<Box<dyn PlayerAgent>> = Some(Box::new(TauriAgent::new_local(
        engine_pid,
        game_id.clone(),
        engine_prompt_tx.clone(),
        engine_response_rx,
        engine_notify_tx,
        engine_snapshot_tx,
    )));

    let mut remote_rx_map: HashMap<usize, mpsc::Receiver<PlayerAction>> =
        remote_response_rxs.into_iter().collect();

    let mut agents: Vec<Box<dyn PlayerAgent>> = Vec::with_capacity(num_players);
    for i in 0..num_players {
        if i == engine_player_index {
            agents.push(engine_agent.take().unwrap());
        } else {
            let pid = PlayerId(i as u32);
            let resp_rx = remote_rx_map
                .remove(&i)
                .expect("Missing response rx for remote player");
            let agent =
                TauriAgent::new_relay(pid, i, game_id.clone(), remote_prompt_tx.clone(), resp_rx);
            agents.push(Box::new(agent));
        }
    }

    for i in 0..num_players {
        let pid = PlayerId(i as u32);
        let deck_list = &deck_lists[i];
        if deck_list.len() == 1 && is_preset_id(&deck_list[0].name) {
            build_preset_deck_for_player(&mut game, &deck_list[0].name, pid);
        } else {
            build_custom_deck(&mut game, pid, deck_list);
        }
    }

    let mut game_loop = GameLoop::new(num_players);
    if std::env::var("FORGE_ENGINE_GAME_LOG").is_err() {
        game_loop.game_log.set_enabled(true);
    }
    game_loop.experimental_restore_snapshot =
        std::env::var("FORGE_ENGINE_RESTORE_SNAPSHOT").is_ok();

    let p0 = PlayerId(0);
    let token_db = get_token_db();
    for (script_name, rules) in token_db.iter() {
        let template = card_rules_to_instance(rules, p0);
        game_loop.register_token(script_name.clone(), template);
    }

    let mut rng = rand::rngs::StdRng::from_entropy();

    let _winner = game_loop.run(&mut game, &mut agents, &mut rng, 5000);

    // Send final game-over prompt to the engine-local player.
    let engine_pid = PlayerId(engine_player_index as u32);
    let engine_final_view =
        GameViewDto::from_engine(&game, &game_loop.mana_pools, engine_pid, &game_id, &[], &[]);
    let _ = engine_prompt_tx.send(AgentPrompt {
        display_events: vec![],
        inner: AgentPromptInner::GameOver {
            game_view: engine_final_view,
        },
    });

    // Send final game-over prompt to each remote player
    for i in 0..num_players {
        if i == engine_player_index {
            continue;
        }
        let pid = PlayerId(i as u32);
        let remote_view =
            GameViewDto::from_engine(&game, &game_loop.mana_pools, pid, &game_id, &[], &[]);
        let _ = remote_prompt_tx.send((
            i,
            AgentPrompt {
                display_events: vec![],
                inner: AgentPromptInner::GameOver {
                    game_view: remote_view,
                },
            },
        ));
    }
}
