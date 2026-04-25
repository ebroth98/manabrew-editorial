use std::sync::Mutex;

use forge_agent_interface::ids_codec::player_slot;
use forge_agent_interface::prompt::AgentPrompt;
use forge_agent_interface::simple_ai::choose_simple_ai_action;
use futures_util::{SinkExt, StreamExt};
use tauri::async_runtime::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use crate::preset_decks::CardIdentity;
use crate::server_client::ServerConnectionConfig;
use forge_server::protocol::{ClientMessage, ServerMessage};

pub struct ClientBotManager {
    bots: Mutex<Vec<BotEntry>>,
}

struct BotEntry {
    username: String,
    handle: JoinHandle<()>,
}

#[derive(Debug, Clone)]
pub struct ClientBotConfig {
    pub connection: ServerConnectionConfig,
    pub room_id: String,
    pub username: String,
    pub deck_name: String,
    pub deck_list: Vec<CardIdentity>,
    pub commander_name: Option<String>,
}

impl ClientBotManager {
    pub fn new() -> Self {
        Self {
            bots: Mutex::new(Vec::new()),
        }
    }

    pub fn spawn_bot(&self, config: ClientBotConfig) -> Result<(), String> {
        let bot_username = config.username.clone();
        let bot_username_for_log = bot_username.clone();
        let handle = tauri::async_runtime::spawn(async move {
            if let Err(error) = run_client_bot(config).await {
                eprintln!(
                    "[client_bot] bot '{}' exited: {}",
                    bot_username_for_log, error
                );
            }
        });
        let mut bots = self.bots.lock().map_err(|e| e.to_string())?;
        bots.push(BotEntry {
            username: bot_username,
            handle,
        });
        Ok(())
    }

    pub fn stop_bot(&self, username: &str) -> bool {
        let mut bots = match self.bots.lock() {
            Ok(b) => b,
            Err(_) => return false,
        };
        if let Some(idx) = bots.iter().position(|b| b.username == username) {
            let entry = bots.remove(idx);
            entry.handle.abort();
            true
        } else {
            false
        }
    }

    pub fn stop_all(&self) {
        if let Ok(mut bots) = self.bots.lock() {
            for entry in bots.drain(..) {
                entry.handle.abort();
            }
        }
    }

    pub fn bot_usernames(&self) -> Vec<String> {
        self.bots
            .lock()
            .map(|bots| bots.iter().map(|b| b.username.clone()).collect())
            .unwrap_or_default()
    }
}

async fn run_client_bot(config: ClientBotConfig) -> Result<(), String> {
    let url = format!(
        "{}://{}:{}",
        if config.connection.port == 443 {
            "wss"
        } else {
            "ws"
        },
        config.connection.host,
        config.connection.port,
    );
    let (socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|error| format!("Failed to connect bot to {}: {}", url, error))?;
    let (mut sink, mut stream) = socket.split();

    send(
        &mut sink,
        &ClientMessage::Authenticate {
            username: config.username.clone(),
            password: config.connection.password.clone(),
        },
    )
    .await?;
    wait_for_auth(&mut stream).await?;

    send(
        &mut sink,
        &ClientMessage::JoinRoom {
            room_id: config.room_id.clone(),
            observe: false,
        },
    )
    .await?;
    wait_until_joined(&mut stream, &config.room_id, &config.username).await?;

    send(
        &mut sink,
        &ClientMessage::SetDeckSelection {
            deck_name: config.deck_name.clone(),
            deck_list: config
                .deck_list
                .iter()
                .map(|card| forge_server::protocol::CardIdentity {
                    name: card.name.clone(),
                    set_code: card.set_code.clone(),
                })
                .collect(),
            commander_name: config.commander_name.clone(),
        },
    )
    .await?;
    send(&mut sink, &ClientMessage::SetReady { ready: true }).await?;

    let mut player_slot_id: Option<String> = None;
    let mut last_choose_action_signature = None;
    let mut last_choose_action_choice = None;
    while let Some(frame) = stream.next().await {
        let frame = frame.map_err(|error| error.to_string())?;
        let Message::Text(text) = frame else {
            continue;
        };
        let message: ServerMessage =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        match message {
            ServerMessage::GameStarted { player_order, .. } => {
                player_slot_id = player_order
                    .iter()
                    .position(|player| player == &config.username)
                    .map(player_slot);
            }
            ServerMessage::StateUpdate { state, .. } => {
                if state.get("kind").and_then(|value| value.as_str()) != Some("prompt") {
                    continue;
                }
                let Some(for_player) = state.get("forPlayer").and_then(|value| value.as_str())
                else {
                    continue;
                };
                if player_slot_id.as_deref() != Some(for_player) {
                    continue;
                }
                let Some(prompt_value) = state.get("prompt") else {
                    continue;
                };
                let prompt: AgentPrompt = match serde_json::from_value(prompt_value.clone()) {
                    Ok(prompt) => prompt,
                    Err(error) => {
                        eprintln!("[client_bot] invalid prompt payload: {}", error);
                        continue;
                    }
                };
                let Some(action) = choose_simple_ai_action(
                    prompt,
                    &mut last_choose_action_signature,
                    &mut last_choose_action_choice,
                ) else {
                    continue;
                };
                send(
                    &mut sink,
                    &ClientMessage::BroadcastState {
                        state: serde_json::json!({
                            "kind": "response",
                            "fromPlayer": for_player,
                            "action": action,
                        }),
                    },
                )
                .await?;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn wait_for_auth(
    stream: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<(), String> {
    while let Some(frame) = stream.next().await {
        let frame = frame.map_err(|error| error.to_string())?;
        let Message::Text(text) = frame else {
            continue;
        };
        if let ServerMessage::AuthResult { success, error, .. } =
            serde_json::from_str(&text).map_err(|error| error.to_string())?
        {
            return if success {
                Ok(())
            } else {
                Err(format!("Bot authentication failed: {:?}", error))
            };
        }
    }
    Err("Relay closed before bot authentication".to_string())
}

async fn wait_until_joined(
    stream: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    room_id: &str,
    username: &str,
) -> Result<(), String> {
    while let Some(frame) = stream.next().await {
        let frame = frame.map_err(|error| error.to_string())?;
        let Message::Text(text) = frame else {
            continue;
        };
        match serde_json::from_str(&text).map_err(|error| error.to_string())? {
            ServerMessage::RoomUpdate { room }
                if room.room_id == room_id
                    && room
                        .players
                        .iter()
                        .any(|player| player.username == username) =>
            {
                return Ok(())
            }
            ServerMessage::Error { message, .. } => return Err(message),
            _ => {}
        }
    }
    Err("Relay closed before bot joined room".to_string())
}

async fn send(
    sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    message: &ClientMessage,
) -> Result<(), String> {
    sink.send(Message::Text(
        serde_json::to_string(message)
            .map_err(|error| error.to_string())?
            .into(),
    ))
    .await
    .map_err(|error| error.to_string())
}
