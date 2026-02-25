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
use tauri::{AppHandle, Emitter, Manager};

use crate::ai_agent::SimpleAiAgent;
use crate::card_db::{card_rules_to_instance, get_token_db};
use crate::game_view_dto::GameViewDto;
use crate::preset_decks::{
    build_ai_opponent, build_custom_deck, build_preset_deck_for_player, build_preset_decks,
    is_preset_id, CardIdentity,
};
use crate::prompt::{AgentPrompt, AgentPromptInner, PlayerAction};
use crate::remote_agent::RemotePlayerAgent;
use crate::server_client::ServerClient;
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
        let (notify_tx, notify_rx) = mpsc::channel::<String>();

        let response_tx_clone = response_tx.clone();

        // Prompt forwarder thread: reads prompts, stores latest, and emits Tauri events
        let app_prompt = app.clone();
        let latest_prompt = self.latest_prompt.clone();
        thread::spawn(move || {
            eprintln!("[prompt_fwd] Prompt forwarder started");
            while let Ok(prompt) = prompt_rx.recv() {
                eprintln!("[prompt_fwd] Got prompt, storing and emitting...");
                if let Ok(mut lp) = latest_prompt.lock() {
                    *lp = Some(prompt.clone());
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
        });

        Ok(game_id)
    }

    pub fn respond(&self, app: AppHandle, action: PlayerAction) -> Result<(), String> {
        if matches!(action, PlayerAction::Concede) {
            // Build a synthetic game-over prompt using the last known game view
            let game_view = {
                let lp = self.latest_prompt.lock().map_err(|e| e.to_string())?;
                let base_view = lp.as_ref().and_then(|p| match &p.inner {
                    AgentPromptInner::ChooseAction { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseAttackers { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseBlockers { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseTargetPlayer { game_view, .. } => {
                        Some(game_view.clone())
                    }
                    AgentPromptInner::ChooseTargetCard { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseTargetAny { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseTargetCardFromZone { game_view, .. } => {
                        Some(game_view.clone())
                    }
                    AgentPromptInner::Mulligan { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::GameOver { game_view } => Some(game_view.clone()),
                    AgentPromptInner::StateUpdate { game_view } => Some(game_view.clone()),
                    AgentPromptInner::Scry { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::Surveil { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::Dig { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseDiscard { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseTargetSpell { game_view, .. } => {
                        Some(game_view.clone())
                    }
                    AgentPromptInner::ChooseMode { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseOptionalTrigger { game_view, .. } => {
                        Some(game_view.clone())
                    }
                    AgentPromptInner::ChooseKicker { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseBuyback { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseMultikicker { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseReplicate { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseAlternativeCost { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseColor { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseCardsForEffect { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseType { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseNumber { game_view, .. } => Some(game_view.clone()),
                    AgentPromptInner::ChooseCardName { game_view, .. } => Some(game_view.clone()),
                });
                let mut view = base_view.unwrap_or_else(|| GameViewDto {
                    game_id: String::new(),
                    turn: 0,
                    step: "main1".into(),
                    combat_assignments: vec![],
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
                    monarch_id: None,
                    initiative_holder_id: None,
                });
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
                    .send(PlayerAction::PlayCard { card_id: None });
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
        host_player_index: usize,
        starting_life: i32,
    ) -> Result<String, String> {
        let num_players = player_names.len();
        if num_players < 2 {
            return Err("Need at least 2 players".into());
        }
        if host_player_index >= num_players {
            return Err("Invalid host player index".into());
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

        let game_id = format!("game-{}", uuid_simple());
        let game_id_clone = game_id.clone();

        // Host player channels (TauriAgent)
        let (host_prompt_tx, host_prompt_rx) = mpsc::channel::<AgentPrompt>();
        let (host_response_tx, host_response_rx) = mpsc::channel::<PlayerAction>();
        let (host_notify_tx, notify_rx) = mpsc::channel::<String>();

        let host_response_tx_clone = host_response_tx.clone();

        // Remote players: shared prompt channel and per-player response channels
        let (remote_prompt_tx, remote_prompt_rx) = mpsc::channel::<(usize, AgentPrompt)>();
        let mut remote_response_txs: HashMap<usize, mpsc::Sender<PlayerAction>> = HashMap::new();
        let mut remote_response_rxs: Vec<(usize, mpsc::Receiver<PlayerAction>)> = Vec::new();

        for i in 0..num_players {
            if i != host_player_index {
                let (resp_tx, resp_rx) = mpsc::channel::<PlayerAction>();
                remote_response_txs.insert(i, resp_tx);
                remote_response_rxs.push((i, resp_rx));
            }
        }

        // Keep clones for the game thread (which builds agents internally)
        let game_host_prompt_tx = host_prompt_tx.clone();
        let game_remote_prompt_tx = remote_prompt_tx.clone();

        // Drop extra senders so forwarders terminate when the game thread ends
        drop(host_prompt_tx);
        drop(remote_prompt_tx);

        // Host prompt forwarder (same as single-player)
        let app_prompt = app.clone();
        let latest_prompt = self.latest_prompt.clone();
        thread::spawn(move || {
            eprintln!("[prompt_fwd] Host prompt forwarder started");
            while let Ok(prompt) = host_prompt_rx.recv() {
                if let Ok(mut lp) = latest_prompt.lock() {
                    *lp = Some(prompt.clone());
                }
                let _ = app_prompt.emit("game:prompt", &prompt);
            }
            eprintln!("[prompt_fwd] Host prompt forwarder ended");
        });

        // Notify forwarder
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

        // Remote prompt forwarder: reads (player_index, prompt) and broadcasts via server
        let app_remote = app.clone();
        thread::spawn(move || {
            eprintln!("[remote_fwd] Remote prompt forwarder started");
            while let Ok((player_index, prompt)) = remote_prompt_rx.recv() {
                let envelope = serde_json::json!({
                    "kind": "prompt",
                    "forPlayer": format!("player-{}", player_index),
                    "prompt": prompt,
                });
                let msg = serde_json::json!({
                    "type": "BroadcastState",
                    "state": envelope,
                });
                if let Some(client) = app_remote.try_state::<ServerClient>() {
                    let _ = client.send(&msg.to_string());
                }
            }
            eprintln!("[remote_fwd] Remote prompt forwarder ended");
        });

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
                    host_player_index,
                    starting_life,
                    game_host_prompt_tx,
                    host_response_rx,
                    host_notify_tx,
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
            response_tx: host_response_tx_clone,
            remote_response_txs,
            thread_handle: Some(handle),
            is_multiplayer: true,
        });

        Ok(game_id)
    }

    /// Route a response from a remote player to the appropriate agent's channel.
    pub fn route_remote_response(&self, state: &serde_json::Value) {
        let from_player = match state.get("fromPlayer").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                eprintln!("[route] Missing fromPlayer in response envelope");
                return;
            }
        };

        // Parse "player-N" → N
        let player_index: usize = match from_player
            .strip_prefix("player-")
            .and_then(|n| n.parse().ok())
        {
            Some(idx) => idx,
            None => {
                eprintln!("[route] Invalid fromPlayer: {}", from_player);
                return;
            }
        };

        let action: PlayerAction = match state.get("action") {
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("[route] Failed to deserialize action: {}", e);
                    return;
                }
            },
            None => {
                eprintln!("[route] Missing action in response envelope");
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
    notify_tx: mpsc::Sender<String>,
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

    let human = TauriAgent::new(
        p0,
        game_id.clone(),
        prompt_tx.clone(),
        response_rx,
        notify_tx,
    );
    let ai = SimpleAiAgent;

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];
    let mut game_loop = GameLoop::new(2);

    // Register token templates so the engine can instantiate tokens by script name.
    // Uses a placeholder owner (p0); the actual owner/controller is set at creation time.
    let token_db = get_token_db();
    for (script_name, rules) in token_db.iter() {
        let template = card_rules_to_instance(rules, p0);
        game_loop.register_token(script_name.clone(), template);
    }

    let mut rng = rand::rngs::StdRng::from_entropy();

    let winner = game_loop.run(&mut game, &mut agents, &mut rng, 50);

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
    host_player_index: usize,
    starting_life: i32,
    host_prompt_tx: mpsc::Sender<AgentPrompt>,
    host_response_rx: mpsc::Receiver<PlayerAction>,
    host_notify_tx: mpsc::Sender<String>,
    remote_prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
    remote_response_rxs: Vec<(usize, mpsc::Receiver<PlayerAction>)>,
) {
    let num_players = player_names.len();
    let name_refs: Vec<&str> = player_names.iter().map(|s| s.as_str()).collect();
    let mut game = GameState::new(&name_refs, starting_life);

    // Build agents inside the thread (avoids Send issues with trait objects).
    let host_pid = PlayerId(host_player_index as u32);
    let mut host_agent: Option<Box<dyn PlayerAgent>> = Some(Box::new(TauriAgent::new(
        host_pid,
        game_id.clone(),
        host_prompt_tx.clone(),
        host_response_rx,
        host_notify_tx,
    )));

    let mut remote_rx_map: HashMap<usize, mpsc::Receiver<PlayerAction>> =
        remote_response_rxs.into_iter().collect();

    let mut agents: Vec<Box<dyn PlayerAgent>> = Vec::with_capacity(num_players);
    for i in 0..num_players {
        if i == host_player_index {
            agents.push(host_agent.take().unwrap());
        } else {
            let pid = PlayerId(i as u32);
            let resp_rx = remote_rx_map
                .remove(&i)
                .expect("Missing response rx for remote player");
            let agent =
                RemotePlayerAgent::new(pid, i, game_id.clone(), remote_prompt_tx.clone(), resp_rx);
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

    let p0 = PlayerId(0);
    let token_db = get_token_db();
    for (script_name, rules) in token_db.iter() {
        let template = card_rules_to_instance(rules, p0);
        game_loop.register_token(script_name.clone(), template);
    }

    let mut rng = rand::rngs::StdRng::from_entropy();

    let _winner = game_loop.run(&mut game, &mut agents, &mut rng, 50);

    // Send final game-over prompt to the host
    let host_pid = PlayerId(host_player_index as u32);
    let host_final_view =
        GameViewDto::from_engine(&game, &game_loop.mana_pools, host_pid, &game_id, &[], &[]);
    let _ = host_prompt_tx.send(AgentPrompt {
        display_events: vec![],
        inner: AgentPromptInner::GameOver {
            game_view: host_final_view,
        },
    });

    // Send final game-over prompt to each remote player
    for i in 0..num_players {
        if i == host_player_index {
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
