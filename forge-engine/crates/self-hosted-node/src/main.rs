#![allow(clippy::too_many_arguments)]
use std::collections::HashMap;
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

mod config;
mod engine_backend;

use config::{workspace_root, Config, DeckSelection};
use engine_backend::{java_backend, rust_backend, EngineBackendKind};
use forge_agent_interface::ids_codec::{parse_player_slot, player_slot};
use forge_agent_interface::java_prompt_normalizer::translate_java_action_value;
use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};
use forge_agent_interface::simple_ai::choose_simple_ai_action;
use forge_engine_core::game::TypeRegistry;
use forge_server::protocol::{
    CardIdentity, ClientMessage, PlayerDeckInfo, RoomInfo, RoomStatus, ServerMessage,
};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWrite = SplitSink<WsStream, Message>;
type WsRead = SplitStream<WsStream>;

const SELF_HOSTED_NODE_PROTOCOL: &str = "self-hosted-node";
const BOT_PROTOCOL: &str = "simple-ai-bot";

#[derive(Debug, Clone)]
enum Role {
    Host,
    Bot,
}

struct RelayClient {
    username: String,
    write: WsWrite,
    read: WsRead,
}

enum EngineSession {
    Rust {
        remote_response_txs: HashMap<usize, std_mpsc::Sender<PlayerAction>>,
    },
    Java {
        remote_response_txs: HashMap<usize, std_mpsc::Sender<Value>>,
    },
}

#[derive(Default)]
struct ClientLoopState {
    player_slot: Option<String>,
    last_choose_action_signature: Option<String>,
    last_choose_action_choice: Option<forge_engine_core::player::actions::PlayerAction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpawnBotPayload {
    #[serde(default)]
    deck: Option<SpawnBotDeckPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpawnBotDeckPayload {
    deck_name: String,
    deck_list: Vec<CardIdentity>,
    #[serde(default)]
    commander_name: Option<String>,
}

type SharedEngineSession = Arc<Mutex<Option<EngineSession>>>;
type SharedBotState = Arc<Mutex<bool>>;
type SharedClientLoopState = Arc<Mutex<ClientLoopState>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "self_hosted_node=info".into()),
        )
        .init();

    if std::env::var("SELF_HOSTED_NODE_JAVA_SMOKE").is_ok() {
        let max_prompts = std::env::var("SELF_HOSTED_NODE_JAVA_SMOKE_PROMPTS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(4);
        if let Err(error) = java_backend::run_smoke_game(max_prompts) {
            error!(%error, "java-forge smoke failed");
            std::process::exit(1);
        }
        info!(max_prompts, "java-forge smoke completed");
        return;
    }
    if let Ok(scenario_name) = std::env::var("SELF_HOSTED_NODE_JAVA_SCENARIO") {
        let max_prompts = std::env::var("SELF_HOSTED_NODE_JAVA_SCENARIO_PROMPTS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(20);
        if let Err(error) = java_backend::run_scenario(&scenario_name, max_prompts) {
            error!(scenario_name, %error, "java-forge scenario failed");
            std::process::exit(1);
        }
        info!(scenario_name, max_prompts, "java-forge scenario completed");
        return;
    }

    let config = Config::from_env();
    if let Err(error) = run(config).await {
        error!(%error, "room node exited");
        std::process::exit(1);
    }
}

async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    load_type_registry()?;

    info!(
        relay_url = %config.relay_url,
        username = %config.username,
        auto_start = config.auto_start,
        engine_enabled = config.engine_enabled,
        host_plays = config.host_plays,
        bot_enabled = config.bot_enabled,
        "starting self-hosted node"
    );

    let mut host =
        RelayClient::connect(&config.relay_url, &config.username, &config.password).await?;

    let room_id = if let Some(room_id) = &config.room_id {
        host.send(&ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            observe: !config.host_plays,
        })
        .await?;
        info!(room_id, "joining configured room");
        room_id.clone()
    } else {
        host.send(&ClientMessage::CreateRoom {
            room_name: config.room_name.clone(),
            max_players: config.max_players,
            format: config.format.clone(),
            hosted: !config.host_plays,
        })
        .await?;
        info!(room_name = %config.room_name, "creating room");
        wait_for_host_room(&mut host, &config).await?
    };

    if config.host_plays {
        seat_client(&mut host, &config.host_deck).await?;
    } else {
        info!(
            username = %config.username,
            "hosting room without occupying a player seat"
        );
    }

    let bot_state: SharedBotState = Arc::new(Mutex::new(false));
    if config.bot_enabled {
        maybe_spawn_bot(config.clone(), room_id.clone(), bot_state.clone());
    }

    let engine_session: SharedEngineSession = Arc::new(Mutex::new(None));
    let loop_state = Arc::new(Mutex::new(ClientLoopState::default()));
    let (outbound_tx, outbound_rx) = tokio_mpsc::unbounded_channel::<ClientMessage>();
    run_client_loop(
        host,
        config.clone(),
        Role::Host,
        room_id,
        engine_session,
        bot_state,
        loop_state,
        outbound_tx,
        outbound_rx,
    )
    .await
}

async fn run_bot(
    config: Config,
    room_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut bot =
        RelayClient::connect(&config.relay_url, &config.bot_username, &config.password).await?;
    bot.send(&ClientMessage::JoinRoom {
        room_id: room_id.clone(),
        observe: false,
    })
    .await?;
    info!(room_id, username = %config.bot_username, "bot joining room");

    wait_until_room_contains(&mut bot, &room_id, &config.bot_username).await?;
    seat_client(&mut bot, &config.bot_deck).await?;
    let engine_session: SharedEngineSession = Arc::new(Mutex::new(None));
    let loop_state = Arc::new(Mutex::new(ClientLoopState::default()));
    let (outbound_tx, outbound_rx) = tokio_mpsc::unbounded_channel::<ClientMessage>();
    run_client_loop(
        bot,
        config,
        Role::Bot,
        room_id,
        engine_session,
        Arc::new(Mutex::new(false)),
        loop_state,
        outbound_tx,
        outbound_rx,
    )
    .await
}

fn maybe_spawn_bot(config: Config, room_id: String, bot_state: SharedBotState) {
    let should_spawn = match bot_state.lock() {
        Ok(mut spawned) => {
            if *spawned {
                false
            } else {
                *spawned = true;
                true
            }
        }
        Err(error) => {
            warn!(%error, "bot state lock poisoned");
            false
        }
    };
    if !should_spawn {
        debug!(room_id, "bot already spawned for room");
        return;
    }

    tokio::spawn(async move {
        if let Err(error) = run_bot(config, room_id).await {
            error!(%error, "bot task exited");
        }
    });
}

fn load_type_registry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let type_lists_path = workspace_root()
        .join("forge")
        .join("forge-gui")
        .join("res")
        .join("lists")
        .join("TypeLists.txt");
    let contents = std::fs::read_to_string(&type_lists_path)?;
    TypeRegistry::load(&contents);
    Ok(())
}

async fn wait_for_host_room(
    client: &mut RelayClient,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match client.recv().await? {
            Some(ServerMessage::RoomCreated { room_id, room_name }) => {
                info!(room_id, room_name, "room created");
                return Ok(room_id);
            }
            Some(ServerMessage::RoomUpdate { room }) if room.host == config.username => {
                info!(room_id = %room.room_id, room_name = %room.room_name, "host room update");
                return Ok(room.room_id);
            }
            Some(ServerMessage::AuthResult { success, error, .. }) => {
                if !success {
                    return Err(format!("authentication failed: {:?}", error).into());
                }
            }
            Some(ServerMessage::Error { code, message }) => {
                return Err(format!("server error while creating room: {code}: {message}").into());
            }
            Some(other) => debug!(?other, "ignored message while waiting for room creation"),
            None => return Err("relay closed while waiting for room creation".into()),
        }
    }
}

async fn wait_until_room_contains(
    client: &mut RelayClient,
    room_id: &str,
    username: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match client.recv().await? {
            Some(ServerMessage::RoomUpdate { room })
                if room.room_id == room_id
                    && room
                        .players
                        .iter()
                        .any(|player| player.username == username) =>
            {
                info!(room_id, username, "client is seated in room");
                return Ok(());
            }
            Some(ServerMessage::AuthResult { success, error, .. }) => {
                if !success {
                    return Err(format!("authentication failed: {:?}", error).into());
                }
            }
            Some(ServerMessage::Error { code, message }) => {
                return Err(format!("server error while joining room: {code}: {message}").into());
            }
            Some(other) => debug!(?other, "ignored message while waiting for room join"),
            None => return Err("relay closed while waiting for room join".into()),
        }
    }
}

async fn seat_client(
    client: &mut RelayClient,
    deck: &DeckSelection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    client
        .send(&ClientMessage::SetDeckSelection {
            deck_name: deck.name.clone(),
            deck_list: deck.cards.clone(),
            commander_name: deck.commander_name.clone(),
        })
        .await?;
    client
        .send(&ClientMessage::SetReady { ready: true })
        .await?;
    info!(
        username = %client.username,
        deck = %deck.name,
        cards = deck.cards.len(),
        "selected deck and marked ready"
    );
    Ok(())
}

async fn run_client_loop(
    mut client: RelayClient,
    config: Config,
    role: Role,
    room_id: String,
    engine_session: SharedEngineSession,
    bot_state: SharedBotState,
    loop_state: SharedClientLoopState,
    outbound_tx: tokio_mpsc::UnboundedSender<ClientMessage>,
    mut outbound_rx: tokio_mpsc::UnboundedReceiver<ClientMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut heartbeat = time::interval(Duration::from_secs(30));
    heartbeat.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                client.broadcast_room_message(protocol_for_role(&role), json!({
                    "type": "heartbeat",
                    "role": role_name(&role),
                    "node": client.username,
                })).await?;
            }
            outbound = outbound_rx.recv() => {
                let Some(outbound) = outbound else {
                    warn!(username = %client.username, "outbound channel closed");
                    return Ok(());
                };
                client.send(&outbound).await?;
            }
            message = client.recv() => {
                let Some(message) = message? else {
                    warn!(username = %client.username, "relay websocket closed");
                    return Ok(());
                };
                handle_server_message(
                    &mut client,
                    &config,
                    &role,
                    &room_id,
                    &engine_session,
                    &bot_state,
                    &loop_state,
                    &outbound_tx,
                    message,
                ).await?;
            }
        }
    }
}

async fn handle_server_message(
    client: &mut RelayClient,
    config: &Config,
    role: &Role,
    room_id: &str,
    engine_session: &SharedEngineSession,
    bot_state: &SharedBotState,
    loop_state: &SharedClientLoopState,
    outbound_tx: &tokio_mpsc::UnboundedSender<ClientMessage>,
    message: ServerMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match message {
        ServerMessage::RoomUpdate { room } => {
            log_room_update(&client.username, &room);
            maybe_auto_start_room(client, config, role, &room).await?;
        }
        ServerMessage::StateUpdate { from_player, state } => {
            handle_state_update(
                client,
                config,
                role,
                room_id,
                engine_session,
                bot_state,
                loop_state,
                from_player,
                state,
            )
            .await?;
        }
        ServerMessage::ReadyStateChanged { username, ready } => {
            info!(username, ready, observer = %client.username, "ready changed");
        }
        ServerMessage::PlayerJoined { username, room_id } => {
            info!(username, room_id, observer = %client.username, "player joined");
        }
        ServerMessage::PlayerLeft { username, room_id } => {
            info!(username, room_id, observer = %client.username, "player left");
        }
        ServerMessage::GameStarted {
            room_id,
            player_order,
            player_decks,
            starting_life,
        } => {
            info!(room_id, ?player_order, observer = %client.username, "game started");
            remember_player_slot(&client.username, role, loop_state, &player_order);
            maybe_start_hosted_engine(
                config,
                role,
                engine_session,
                outbound_tx,
                player_order,
                player_decks,
                starting_life,
            );
        }
        ServerMessage::Error { code, message } => {
            warn!(code, message, observer = %client.username, "server error");
        }
        other => {
            debug!(?other, observer = %client.username, "server message");
        }
    }

    Ok(())
}

async fn handle_state_update(
    client: &mut RelayClient,
    config: &Config,
    role: &Role,
    room_id: &str,
    engine_session: &SharedEngineSession,
    bot_state: &SharedBotState,
    loop_state: &SharedClientLoopState,
    from_player: String,
    state: Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if state.get("kind").and_then(Value::as_str) == Some("response") {
        route_remote_response(engine_session, &state);
        return Ok(());
    }

    if state.get("kind").and_then(Value::as_str) == Some("prompt") {
        maybe_answer_bot_prompt(client, role, loop_state, &state).await?;
        return Ok(());
    }

    if state.get("kind").and_then(Value::as_str) != Some("roomRelay") {
        debug!(from_player, state = %state, observer = %client.username, "state update");
        return Ok(());
    }

    let protocol = state
        .get("protocol")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    info!(
        from_player,
        protocol,
        state = %state,
        observer = %client.username,
        "room relay message"
    );

    let payload_type = state
        .get("payload")
        .and_then(|payload| payload.get("type"))
        .and_then(Value::as_str);

    if protocol == SELF_HOSTED_NODE_PROTOCOL && payload_type == Some("removeBot") {
        if matches!(role, Role::Host) {
            match bot_state.lock() {
                Ok(mut spawned) => *spawned = false,
                Err(error) => warn!(%error, "bot state lock poisoned"),
            }
        }
        if matches!(role, Role::Bot) {
            info!(observer = %client.username, "received self-hosted-node removeBot request");
            client.send(&ClientMessage::LeaveRoom).await?;
        }
    }

    if protocol == SELF_HOSTED_NODE_PROTOCOL
        && payload_type == Some("startGame")
        && matches!(role, Role::Host)
    {
        info!(observer = %client.username, "received self-hosted-node startGame request");
        client.send(&ClientMessage::StartGame).await?;
    }

    if protocol == SELF_HOSTED_NODE_PROTOCOL
        && payload_type == Some("spawnBot")
        && matches!(role, Role::Host)
    {
        let requested_room_id = state
            .get("roomId")
            .and_then(Value::as_str)
            .unwrap_or(room_id);
        if requested_room_id == room_id {
            info!(observer = %client.username, room_id, "received self-hosted-node spawnBot request");
            let bot_config = config_for_spawn_bot(config, &state);
            maybe_spawn_bot(bot_config, room_id.to_string(), bot_state.clone());
        } else {
            debug!(
                observer = %client.username,
                current_room_id = room_id,
                requested_room_id,
                "ignoring spawnBot request for another room"
            );
        }
    }

    if protocol == BOT_PROTOCOL && payload_type == Some("ping") && matches!(role, Role::Bot) {
        client
            .broadcast_room_message(
                BOT_PROTOCOL,
                json!({
                    "type": "pong",
                    "bot": client.username,
                }),
            )
            .await?;
    }

    Ok(())
}

fn remember_player_slot(
    username: &str,
    role: &Role,
    loop_state: &SharedClientLoopState,
    player_order: &[String],
) {
    if !matches!(role, Role::Bot) {
        return;
    }
    let slot = player_order
        .iter()
        .position(|player| player == username)
        .map(player_slot);
    let Some(slot) = slot else {
        warn!(
            username,
            ?player_order,
            "bot was not present in game player order"
        );
        return;
    };
    match loop_state.lock() {
        Ok(mut state) => {
            state.player_slot = Some(slot.clone());
            info!(username, player_slot = %slot, "bot player slot assigned");
        }
        Err(error) => warn!(%error, "client loop state lock poisoned"),
    }
}

async fn maybe_answer_bot_prompt(
    client: &mut RelayClient,
    role: &Role,
    loop_state: &SharedClientLoopState,
    state: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !matches!(role, Role::Bot) {
        return Ok(());
    }

    let Some(for_player) = state.get("forPlayer").and_then(Value::as_str) else {
        debug!(observer = %client.username, state = %state, "bot prompt missing forPlayer");
        return Ok(());
    };
    let Some(prompt_value) = state.get("prompt") else {
        debug!(observer = %client.username, state = %state, "bot prompt missing prompt body");
        return Ok(());
    };

    let action_value = {
        let mut guard = match loop_state.lock() {
            Ok(guard) => guard,
            Err(error) => {
                warn!(%error, "client loop state lock poisoned");
                return Ok(());
            }
        };
        if guard.player_slot.as_deref() != Some(for_player) {
            debug!(
                observer = %client.username,
                for_player,
                bot_slot = ?guard.player_slot,
                "ignoring prompt for another player"
            );
            return Ok(());
        }
        if prompt_value.get("kind").and_then(Value::as_str) == Some("priority") {
            Some(json!({ "kind": "pass" }))
        } else if let Some(action) = choose_simple_ai_action_from_prompt_value(prompt_value) {
            match serde_json::to_value(action) {
                Ok(value) => Some(value),
                Err(error) => {
                    warn!(observer = %client.username, %error, "failed to serialize bot action");
                    None
                }
            }
        } else {
            let prompt: AgentPrompt = match serde_json::from_value(prompt_value.clone()) {
                Ok(prompt) => prompt,
                Err(error) => {
                    warn!(observer = %client.username, %error, "bot prompt payload was invalid");
                    return Ok(());
                }
            };
            let mut signature = guard.last_choose_action_signature.take();
            let mut choice = guard.last_choose_action_choice.take();
            let action = choose_simple_ai_action(prompt, &mut signature, &mut choice);
            guard.last_choose_action_signature = signature;
            guard.last_choose_action_choice = choice;
            action.and_then(|action| match serde_json::to_value(action) {
                Ok(value) => Some(value),
                Err(error) => {
                    warn!(observer = %client.username, %error, "failed to serialize bot action");
                    None
                }
            })
        }
    };

    let Some(action_value) = action_value else {
        warn!(observer = %client.username, for_player, "simple AI had no response for prompt");
        return Ok(());
    };

    client
        .send(&ClientMessage::BroadcastState {
            state: json!({
                "kind": "response",
                "fromPlayer": for_player,
                "action": action_value,
            }),
        })
        .await?;
    info!(observer = %client.username, for_player, "bot answered prompt");
    Ok(())
}

fn choose_simple_ai_action_from_prompt_value(prompt: &Value) -> Option<PlayerAction> {
    match prompt.get("type").and_then(Value::as_str)? {
        "mulligan" => Some(PlayerAction::MulliganDecision { keep: true }),
        "mulliganPutBack" => {
            let count = prompt
                .get("count")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            let card_ids = prompt
                .get("handCardIds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .take(count)
                .map(str::to_string)
                .collect();
            Some(PlayerAction::MulliganPutBackDecision { card_ids })
        }
        _ => None,
    }
}

fn config_for_spawn_bot(config: &Config, state: &Value) -> Config {
    let mut config = config.clone();
    let deck = state
        .get("payload")
        .and_then(|payload| serde_json::from_value::<SpawnBotPayload>(payload.clone()).ok())
        .and_then(|payload| payload.deck);
    if let Some(deck) = deck {
        info!(
            deck = %deck.deck_name,
            cards = deck.deck_list.len(),
            commander = ?deck.commander_name,
            "using requested bot deck"
        );
        config.bot_deck = DeckSelection {
            name: deck.deck_name,
            cards: deck.deck_list,
            commander_name: deck.commander_name,
        };
    }
    config
}

async fn maybe_auto_start_room(
    client: &mut RelayClient,
    config: &Config,
    role: &Role,
    room: &RoomInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !config.auto_start || !matches!(role, Role::Host) {
        return Ok(());
    }
    if room.host != config.username
        || room.status != RoomStatus::Lobby
        || room.players.len() < config.max_players as usize
    {
        return Ok(());
    }
    let all_ready = room
        .players
        .iter()
        .all(|player| player.connected && player.ready && player.selected_deck_name.is_some());
    if all_ready {
        info!(
            room_id = %room.room_id,
            players = room.players.len(),
            "all players ready; auto-starting hosted game"
        );
        client.send(&ClientMessage::StartGame).await?;
    }
    Ok(())
}

fn maybe_start_hosted_engine(
    config: &Config,
    role: &Role,
    engine_session: &SharedEngineSession,
    outbound_tx: &tokio_mpsc::UnboundedSender<ClientMessage>,
    player_order: Vec<String>,
    player_decks: Vec<PlayerDeckInfo>,
    starting_life: i32,
) {
    if !matches!(role, Role::Host) {
        return;
    }
    if !config.engine_enabled {
        debug!("hosted engine disabled for this node");
        return;
    }
    let backend = EngineBackendKind::from_env();
    if !backend.is_supported() {
        if let Err(error) = java_backend::JavaRuntimeConfig::from_env().validate() {
            warn!(
                backend = backend.label(),
                %error,
                "hosted engine backend runtime is not ready"
            );
            return;
        }
        warn!(
            backend = backend.label(),
            message = java_backend::unsupported_message(),
            "hosted engine backend is not implemented yet"
        );
        return;
    }

    let local_player_index = if config.host_plays {
        match player_order
            .iter()
            .position(|name| name == &config.username)
        {
            Some(index) => Some(index),
            None => {
                warn!(
                    username = %config.username,
                    ?player_order,
                    "room node is configured as a player but is not in player order; not starting engine"
                );
                return;
            }
        }
    } else {
        None
    };

    let mut guard = match engine_session.lock() {
        Ok(guard) => guard,
        Err(error) => {
            warn!(%error, "engine session lock poisoned");
            return;
        }
    };
    if guard.is_some() {
        debug!("engine session already running");
        return;
    }

    let num_players = player_order.len();
    if num_players < 2 {
        warn!(num_players, "not enough players to start hosted engine");
        return;
    }

    let mut deck_map: HashMap<String, PlayerDeckInfo> = player_decks
        .into_iter()
        .map(|deck| (deck.username.clone(), deck))
        .collect();
    let mut ordered_decks = Vec::with_capacity(num_players);
    let mut commander_names = Vec::with_capacity(num_players);
    for username in &player_order {
        let Some(deck) = deck_map.remove(username) else {
            warn!(username, "missing deck for player; not starting engine");
            return;
        };
        ordered_decks.push(deck.deck_list);
        commander_names.push(deck.commander_name);
    }

    let player_names = player_order;
    let game_id = format!("room-game-{}", Uuid::new_v4());

    match backend {
        EngineBackendKind::Rust => {
            let (remote_prompt_tx, remote_prompt_rx) = std_mpsc::channel::<(usize, AgentPrompt)>();
            let mut remote_response_txs = HashMap::new();
            let mut remote_response_rxs = Vec::new();
            for i in 0..num_players {
                if Some(i) == local_player_index {
                    continue;
                }
                let (response_tx, response_rx) = std_mpsc::channel::<PlayerAction>();
                remote_response_txs.insert(i, response_tx);
                remote_response_rxs.push((i, response_rx));
            }
            *guard = Some(EngineSession::Rust {
                remote_response_txs,
            });
            drop(guard);

            spawn_remote_prompt_forwarder(outbound_tx.clone(), remote_prompt_rx);
            thread::spawn(move || {
                info!(
                    game_id,
                    backend = backend.label(),
                    players = num_players,
                    local_player_index,
                    "starting hosted engine thread"
                );
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    rust_backend::run_hosted_engine_game(
                        game_id.clone(),
                        player_names,
                        ordered_decks,
                        commander_names,
                        local_player_index,
                        starting_life,
                        remote_prompt_tx,
                        remote_response_rxs,
                    )
                }));
                log_hosted_engine_result(result);
            });
        }
        EngineBackendKind::JavaForge => {
            let (remote_prompt_tx, remote_prompt_rx) = std_mpsc::channel::<(usize, Value)>();
            let mut remote_response_txs = HashMap::new();
            let mut remote_response_rxs = Vec::new();
            for i in 0..num_players {
                if Some(i) == local_player_index {
                    continue;
                }
                let (response_tx, response_rx) = std_mpsc::channel::<Value>();
                remote_response_txs.insert(i, response_tx);
                remote_response_rxs.push((i, response_rx));
            }
            *guard = Some(EngineSession::Java {
                remote_response_txs,
            });
            drop(guard);

            spawn_raw_prompt_forwarder(outbound_tx.clone(), remote_prompt_rx);
            thread::spawn(move || {
                info!(
                    game_id,
                    backend = backend.label(),
                    players = num_players,
                    local_player_index,
                    "starting hosted engine thread"
                );
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    java_backend::run_hosted_engine_game(
                        game_id.clone(),
                        player_names,
                        ordered_decks,
                        commander_names,
                        local_player_index,
                        starting_life,
                        remote_prompt_tx,
                        remote_response_rxs,
                    )
                }));
                log_hosted_engine_result(result);
            });
        }
    }
}

fn log_hosted_engine_result(result: std::thread::Result<()>) {
    match result {
        Ok(()) => info!("hosted engine thread finished"),
        Err(error) => {
            let message = if let Some(message) = error.downcast_ref::<String>() {
                message.clone()
            } else if let Some(message) = error.downcast_ref::<&str>() {
                message.to_string()
            } else {
                "unknown panic".to_string()
            };
            error!(message, "hosted engine thread panicked");
        }
    }
}

fn route_remote_response(engine_session: &SharedEngineSession, state: &Value) {
    let from_player = match state.get("fromPlayer").and_then(Value::as_str) {
        Some(from_player) => from_player,
        None => {
            warn!(state = %state, "relay response missing fromPlayer");
            return;
        }
    };
    let Some(player_index) = parse_player_slot(from_player) else {
        warn!(from_player, "relay response has invalid player slot");
        return;
    };
    let Some(action_value) = state.get("action") else {
        warn!(from_player, "relay response missing action");
        return;
    };

    let guard = match engine_session.lock() {
        Ok(guard) => guard,
        Err(error) => {
            warn!(%error, "engine session lock poisoned");
            return;
        }
    };
    let Some(session) = guard.as_ref() else {
        debug!(from_player, "no engine session for relay response");
        return;
    };
    match session {
        EngineSession::Rust {
            remote_response_txs,
        } => {
            let action: PlayerAction = match serde_json::from_value(action_value.clone()) {
                Ok(action) => action,
                Err(error) => {
                    warn!(from_player, %error, "relay response has invalid rust action");
                    return;
                }
            };
            let Some(tx) = remote_response_txs.get(&player_index) else {
                debug!(from_player, player_index, "no response channel for player");
                return;
            };
            if let Err(error) = tx.send(action) {
                warn!(from_player, %error, "failed to route relay response");
            }
        }
        EngineSession::Java {
            remote_response_txs,
        } => {
            let Some(tx) = remote_response_txs.get(&player_index) else {
                debug!(from_player, player_index, "no response channel for player");
                return;
            };
            if let Err(error) = tx.send(translate_java_action_value(action_value)) {
                warn!(from_player, %error, "failed to route relay response");
            }
        }
    }
}

fn spawn_remote_prompt_forwarder(
    outbound_tx: tokio_mpsc::UnboundedSender<ClientMessage>,
    remote_prompt_rx: std_mpsc::Receiver<(usize, AgentPrompt)>,
) {
    thread::spawn(move || {
        while let Ok((player_index, prompt)) = remote_prompt_rx.recv() {
            let state = json!({
                "kind": "prompt",
                "forPlayer": player_slot(player_index),
                "prompt": prompt,
            });
            if outbound_tx
                .send(ClientMessage::BroadcastState { state })
                .is_err()
            {
                break;
            }
        }
    });
}

fn spawn_raw_prompt_forwarder(
    outbound_tx: tokio_mpsc::UnboundedSender<ClientMessage>,
    remote_prompt_rx: std_mpsc::Receiver<(usize, Value)>,
) {
    thread::spawn(move || {
        while let Ok((player_index, prompt)) = remote_prompt_rx.recv() {
            let state = json!({
                "kind": "prompt",
                "forPlayer": player_slot(player_index),
                "prompt": prompt,
            });
            if outbound_tx
                .send(ClientMessage::BroadcastState { state })
                .is_err()
            {
                break;
            }
        }
    });
}

impl RelayClient {
    async fn connect(
        relay_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!(relay_url, username, "connecting relay client");
        let (socket, _) = connect_async(relay_url).await?;
        let (write, read) = socket.split();
        let mut client = Self {
            username: username.to_string(),
            write,
            read,
        };
        client
            .send(&ClientMessage::Authenticate {
                username: username.to_string(),
                password: password.to_string(),
            })
            .await?;
        client.wait_for_auth().await?;
        Ok(client)
    }

    async fn wait_for_auth(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            match self.recv().await? {
                Some(ServerMessage::AuthResult { success, error, .. }) => {
                    if success {
                        info!(username = %self.username, "authenticated");
                        return Ok(());
                    }
                    return Err(format!("authentication failed: {:?}", error).into());
                }
                Some(other) => {
                    debug!(?other, username = %self.username, "ignored pre-auth message")
                }
                None => return Err("relay closed before authentication".into()),
            }
        }
    }

    async fn send(
        &mut self,
        message: &ClientMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.write
            .send(Message::Text(serde_json::to_string(message)?))
            .await?;
        Ok(())
    }

    async fn recv(
        &mut self,
    ) -> Result<Option<ServerMessage>, Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let Some(message) = self.read.next().await else {
                return Ok(None);
            };
            let message = message?;
            match message {
                Message::Text(text) => {
                    return Ok(Some(serde_json::from_str(&text)?));
                }
                Message::Ping(payload) => {
                    self.write.send(Message::Pong(payload)).await?;
                }
                Message::Close(_) => return Ok(None),
                _ => {}
            }
        }
    }

    async fn broadcast_room_message(
        &mut self,
        protocol: &str,
        payload: Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send(&ClientMessage::BroadcastState {
            state: json!({
                "kind": "roomRelay",
                "protocol": protocol,
                "version": 1,
                "messageId": Uuid::new_v4().to_string(),
                "fromPlayer": self.username,
                "payload": payload,
            }),
        })
        .await
    }
}

fn log_room_update(observer: &str, room: &RoomInfo) {
    let players = room
        .players
        .iter()
        .map(|player| {
            format!(
                "{}{}{}",
                player.username,
                if player.ready { ":ready" } else { "" },
                if player.connected { "" } else { ":offline" },
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    info!(
        observer,
        room_id = %room.room_id,
        room_name = %room.room_name,
        host = %room.host,
        players,
        "room update"
    );
}

fn protocol_for_role(role: &Role) -> &'static str {
    match role {
        Role::Host => SELF_HOSTED_NODE_PROTOCOL,
        Role::Bot => BOT_PROTOCOL,
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Host => "host",
        Role::Bot => "bot",
    }
}
