use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

use crate::error::ServerError;
use crate::lobby;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::state::{ConnectedPlayer, ServerState, WsSender};

/// Send a `ServerMessage` to a single WebSocket sender.
pub async fn send_msg(sender: &Arc<Mutex<WsSender>>, msg: &ServerMessage) {
    if let Ok(json) = serde_json::to_string(msg) {
        let mut tx = sender.lock().await;
        let _ = tx.send(Message::Text(json.into())).await;
    }
}

/// Send a `ServerMessage` to a single player by ID, with logging.
async fn emit_to(state: &Arc<ServerState>, player_id: &str, msg: &ServerMessage, json: &str) {
    if let Some(player) = state.players.get(player_id) {
        if player.connected {
            debug!("[emit] -> '{}': {}", player.username, msg_type_of(msg));
            let mut tx = player.sender.lock().await;
            let _ = tx.send(Message::Text(json.to_string().into())).await;
        }
    }
}

/// Send a `ServerMessage` to every *connected* player in a room, except the sender.
pub async fn broadcast_to_room_except(
    state: &Arc<ServerState>,
    sender_player_id: &str,
    room_id: &str,
    msg: &ServerMessage,
) {
    let json = match serde_json::to_string(msg) {
        Ok(j) => j,
        Err(_) => return,
    };

    let player_ids: Vec<String> = {
        if let Some(room) = state.rooms.get(room_id) {
            room.connected_player_ids()
        } else {
            return;
        }
    };

    let sender_name = get_username(state, sender_player_id);
    let target_count = player_ids.iter().filter(|p| p.as_str() != sender_player_id).count();
    debug!(
        "[broadcast] room={} from='{}' type={} targets={}",
        &room_id[..8],
        sender_name,
        msg_type_of(msg),
        target_count,
    );

    for pid in &player_ids {
        if pid == sender_player_id {
            continue;
        }
        emit_to(state, pid, msg, &json).await;
    }
}

/// Send a `ServerMessage` to every *connected* player in a room (including the sender).
pub async fn broadcast_to_room(
    state: &Arc<ServerState>,
    room_id: &str,
    msg: &ServerMessage,
) {
    let json = match serde_json::to_string(msg) {
        Ok(j) => j,
        Err(_) => return,
    };

    let player_ids: Vec<String> = {
        if let Some(room) = state.rooms.get(room_id) {
            room.connected_player_ids()
        } else {
            return;
        }
    };

    debug!(
        "[broadcast] room={} type={} targets={}",
        &room_id[..8.min(room_id.len())],
        msg_type_of(msg),
        player_ids.len(),
    );

    for pid in &player_ids {
        emit_to(state, pid, msg, &json).await;
    }
}

/// Handle a single WebSocket connection from accept to close.
pub async fn handle_connection(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    state: Arc<ServerState>,
) -> Result<(), ServerError> {
    info!("[connect] new TCP connection from {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(ServerError::WebSocket)?;

    info!("[connect] WebSocket upgraded for {}", addr);

    let (sender, mut receiver) = ws_stream.split();
    let sender = Arc::new(Mutex::new(sender));

    let (player_id, username, reconnected) =
        match authenticate(&mut receiver, &sender, &state).await {
            Ok(result) => result,
            Err(e) => {
                warn!("[auth] failed from {}: {}", addr, e);
                return Err(e);
            }
        };

    if reconnected {
        info!("[auth] '{}' reconnected from {} (id={})", username, addr, &player_id[..8]);
    } else {
        info!("[auth] '{}' authenticated from {} (id={})", username, addr, &player_id[..8]);
    }

    while let Some(frame) = receiver.next().await {
        let frame = match frame {
            Ok(f) => f,
            Err(e) => {
                warn!("[recv] read error from '{}': {}", username, e);
                break;
            }
        };

        match frame {
            Message::Text(text) => {
                let client_msg: ClientMessage = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("[recv] parse error from '{}': {}", username, e);
                        let err_msg = ServerMessage::Error {
                            code: "parse_error".into(),
                            message: e.to_string(),
                        };
                        send_msg(&sender, &err_msg).await;
                        continue;
                    }
                };
                debug!("[recv] '{}' -> {}", username, client_msg_type(&client_msg));
                handle_client_message(&state, &player_id, &username, &sender, client_msg).await;
            }
            Message::Close(_) => {
                info!("[recv] '{}' sent close frame", username);
                break;
            }
            Message::Ping(_) => {
                debug!("[recv] '{}' ping", username);
                let mut tx = sender.lock().await;
                let _ = tx.send(Message::Pong(vec![].into())).await;
            }
            _ => {}
        }
    }

    info!("[disconnect] '{}' (id={})", username, &player_id[..8]);
    mark_disconnected(&state, &player_id, &sender).await;
    Ok(())
}

type WsReceiver = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
>;

async fn authenticate(
    receiver: &mut WsReceiver,
    sender: &Arc<Mutex<WsSender>>,
    state: &Arc<ServerState>,
) -> Result<(String, String, bool), ServerError> {
    let timeout = Duration::from_secs(10);

    let frame = tokio::time::timeout(timeout, receiver.next())
        .await
        .map_err(|_| ServerError::AuthTimeout)?
        .ok_or(ServerError::AuthFailed("Connection closed".into()))?
        .map_err(ServerError::WebSocket)?;

    let text = match frame {
        Message::Text(t) => t,
        _ => {
            return Err(ServerError::AuthFailed("Expected text frame".into()));
        }
    };

    let msg: ClientMessage =
        serde_json::from_str(&text).map_err(|e| ServerError::AuthFailed(e.to_string()))?;

    match msg {
        ClientMessage::Authenticate { username, password } => {
            if password != state.server_key {
                let reply = ServerMessage::AuthResult {
                    success: false,
                    player_id: None,
                    reconnected: None,
                    error: Some("Invalid server key".into()),
                };
                send_msg(sender, &reply).await;
                return Err(ServerError::AuthFailed("Invalid server key".into()));
            }

            if username.trim().is_empty() {
                let reply = ServerMessage::AuthResult {
                    success: false,
                    player_id: None,
                    reconnected: None,
                    error: Some("Username cannot be empty".into()),
                };
                send_msg(sender, &reply).await;
                return Err(ServerError::AuthFailed("Empty username".into()));
            }

            // Check for reconnection
            if let Some((existing_pid, room_id)) = state.find_disconnected_by_username(&username) {
                info!("[auth] reclaiming session for '{}' (id={})", username, &existing_pid[..8]);

                if let Some(mut player) = state.players.get_mut(&existing_pid) {
                    player.sender = sender.clone();
                    player.connected = true;
                }

                // Send AuthResult to the reconnecting client FIRST
                let reply = ServerMessage::AuthResult {
                    success: true,
                    player_id: Some(existing_pid.clone()),
                    reconnected: Some(true),
                    error: None,
                };
                send_msg(sender, &reply).await;

                // Then update room state and notify others
                if let Some(rid) = &room_id {
                    if let Some(mut room) = state.rooms.get_mut(rid) {
                        room.set_connected(&existing_pid, true);
                    }

                    broadcast_to_room_except(
                        state,
                        &existing_pid,
                        rid,
                        &ServerMessage::PlayerConnected {
                            username: username.clone(),
                        },
                    )
                    .await;

                    if let Some(room) = state.rooms.get(rid) {
                        broadcast_to_room(
                            state,
                            rid,
                            &ServerMessage::RoomUpdate {
                                room: room.to_room_info(),
                            },
                        )
                        .await;
                    }
                }

                return Ok((existing_pid, username, true));
            }

            // TODO: Per ora username unique quindi n'se po' ripetere
            if state.username_taken_by_connected(&username) {
                let reply = ServerMessage::AuthResult {
                    success: false,
                    player_id: None,
                    reconnected: None,
                    error: Some(format!("Username '{}' is already taken", username)),
                };
                send_msg(sender, &reply).await;
                return Err(ServerError::DuplicateUsername(username));
            }

            let player_id = uuid::Uuid::new_v4().to_string();
            state.players.insert(
                player_id.clone(),
                ConnectedPlayer {
                    player_id: player_id.clone(),
                    username: username.clone(),
                    room_id: None,
                    sender: sender.clone(),
                    connected: true,
                },
            );

            let reply = ServerMessage::AuthResult {
                success: true,
                player_id: Some(player_id.clone()),
                reconnected: Some(false),
                error: None,
            };
            send_msg(sender, &reply).await;

            Ok((player_id, username, false))
        }
        _ => {
            let reply = ServerMessage::AuthResult {
                success: false,
                player_id: None,
                reconnected: None,
                error: Some("First message must be Authenticate".into()),
            };
            send_msg(sender, &reply).await;
            Err(ServerError::AuthFailed(
                "First message was not Authenticate".into(),
            ))
        }
    }
}

async fn handle_client_message(
    state: &Arc<ServerState>,
    player_id: &str,
    username: &str,
    sender: &Arc<Mutex<WsSender>>,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::Authenticate { .. } => {
            warn!("[recv] '{}' sent Authenticate while already authenticated", username);
            send_msg(
                sender,
                &ServerMessage::Error {
                    code: "already_authenticated".into(),
                    message: "You are already authenticated".into(),
                },
            )
            .await;
        }

        ClientMessage::ListRooms => {
            let rooms: Vec<_> = state
                .rooms
                .iter()
                .map(|entry| entry.value().to_room_info())
                .collect();
            debug!("[emit] -> '{}': RoomList ({} rooms)", username, rooms.len());
            send_msg(sender, &ServerMessage::RoomList { rooms }).await;
        }

        ClientMessage::ListPlayers => {
            let players: Vec<_> = state
                .players
                .iter()
                .map(|entry| crate::protocol::PlayerInfo {
                    username: entry.value().username.clone(),
                    player_id: entry.value().player_id.clone(),
                    connected: entry.value().connected,
                    room_id: entry.value().room_id.clone(),
                })
                .collect();
            debug!("[emit] -> '{}': PlayerList ({} players)", username, players.len());
            send_msg(sender, &ServerMessage::PlayerList { players }).await;
        }

        ClientMessage::CreateRoom {
            room_name,
            max_players,
        } => {
            info!("[lobby] '{}' creating room '{}' (max={})", username, room_name, max_players);
            match lobby::create_room(state, player_id, room_name, max_players).await {
                Ok(info) => {
                    info!("[lobby] room created: {} (id={})", info.room_name, &info.room_id[..8]);
                    send_msg(
                        sender,
                        &ServerMessage::RoomCreated {
                            room_id: info.room_id.clone(),
                            room_name: info.room_name.clone(),
                        },
                    )
                    .await;
                    send_msg(sender, &ServerMessage::RoomUpdate { room: info }).await;
                }
                Err(e) => {
                    warn!("[lobby] '{}' create room failed: {}", username, e);
                    send_msg(
                        sender,
                        &ServerMessage::Error {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            }
        }

        ClientMessage::JoinRoom { room_id } => {
            info!("[lobby] '{}' joining room {}", username, &room_id[..8.min(room_id.len())]);
            match lobby::join_room(state, player_id, &room_id).await {
                Ok(info) => {
                    info!("[lobby] '{}' joined room '{}'", username, info.room_name);
                    broadcast_to_room(
                        state,
                        &room_id,
                        &ServerMessage::PlayerJoined {
                            room_id: room_id.clone(),
                            username: username.to_string(),
                        },
                    )
                    .await;
                    broadcast_to_room(state, &room_id, &ServerMessage::RoomUpdate { room: info })
                        .await;
                }
                Err(e) => {
                    warn!("[lobby] '{}' join room failed: {}", username, e);
                    send_msg(
                        sender,
                        &ServerMessage::Error {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            }
        }

        ClientMessage::LeaveRoom => {
            let room_id_before = state
                .players
                .get(player_id)
                .and_then(|p| p.room_id.clone());

            info!("[lobby] '{}' leaving room", username);
            match lobby::leave_room(state, player_id).await {
                Ok(()) => {
                    if let Some(rid) = room_id_before {
                        info!("[lobby] '{}' left room {}", username, &rid[..8]);
                        broadcast_to_room(
                            state,
                            &rid,
                            &ServerMessage::PlayerLeft {
                                room_id: rid.clone(),
                                username: username.to_string(),
                            },
                        )
                        .await;
                        if let Some(room) = state.rooms.get(&rid) {
                            broadcast_to_room(
                                state,
                                &rid,
                                &ServerMessage::RoomUpdate {
                                    room: room.to_room_info(),
                                },
                            )
                            .await;
                        }
                    }
                }
                Err(e) => {
                    warn!("[lobby] '{}' leave room failed: {}", username, e);
                    send_msg(
                        sender,
                        &ServerMessage::Error {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            }
        }

        ClientMessage::SetReady { ready } => {
            info!("[lobby] '{}' set ready={}", username, ready);
            match lobby::set_ready(state, player_id, ready).await {
                Ok(room_id) => {
                    broadcast_to_room(
                        state,
                        &room_id,
                        &ServerMessage::ReadyStateChanged {
                            username: username.to_string(),
                            ready,
                        },
                    )
                    .await;
                    if let Some(room) = state.rooms.get(&room_id) {
                        broadcast_to_room(
                            state,
                            &room_id,
                            &ServerMessage::RoomUpdate {
                                room: room.to_room_info(),
                            },
                        )
                        .await;
                    }
                }
                Err(e) => {
                    warn!("[lobby] '{}' set ready failed: {}", username, e);
                    send_msg(
                        sender,
                        &ServerMessage::Error {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            }
        }

        ClientMessage::StartGame => {
            info!("[game] '{}' starting game", username);
            match lobby::start_game(state, player_id).await {
                Ok((room_id, player_order)) => {
                    info!("[game] game started in room {} | order: {:?}", &room_id[..8], player_order);
                    broadcast_to_room(
                        state,
                        &room_id,
                        &ServerMessage::GameStarted {
                            room_id: room_id.clone(),
                            player_order,
                        },
                    )
                    .await;
                }
                Err(e) => {
                    warn!("[game] '{}' start game failed: {}", username, e);
                    send_msg(
                        sender,
                        &ServerMessage::Error {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            }
        }

        ClientMessage::BroadcastState { state: game_state } => {
            let room_id = {
                state.players.get(player_id).and_then(|p| p.room_id.clone())
            };
            if let Some(rid) = room_id {
                debug!("[game] '{}' broadcasting state to room {}", username, &rid[..8]);
                broadcast_to_room_except(
                    state,
                    player_id,
                    &rid,
                    &ServerMessage::StateUpdate {
                        from_player: username.to_string(),
                        state: game_state,
                    },
                )
                .await;
            } else {
                warn!("[game] '{}' tried to broadcast state but not in a room", username);
                send_msg(
                    sender,
                    &ServerMessage::Error {
                        code: "not_in_room".into(),
                        message: "You are not in a room".into(),
                    },
                )
                .await;
            }
        }

        ClientMessage::TurnChange {
            new_active_player,
            turn_number,
        } => {
            let room_id = {
                state.players.get(player_id).and_then(|p| p.room_id.clone())
            };
            if let Some(rid) = room_id {
                info!(
                    "[game] turn change in room {}: '{}' -> '{}' (turn {})",
                    &rid[..8], username, new_active_player, turn_number
                );
                broadcast_to_room_except(
                    state,
                    player_id,
                    &rid,
                    &ServerMessage::TurnChanged {
                        from_player: username.to_string(),
                        new_active_player,
                        turn_number,
                    },
                )
                .await;
            } else {
                warn!("[game] '{}' tried turn change but not in a room", username);
                send_msg(
                    sender,
                    &ServerMessage::Error {
                        code: "not_in_room".into(),
                        message: "You are not in a room".into(),
                    },
                )
                .await;
            }
        }

    }
}

async fn mark_disconnected(
    state: &Arc<ServerState>,
    player_id: &str,
    our_sender: &Arc<Mutex<WsSender>>,
) {
    let (username, room_id) = {
        if let Some(mut player) = state.players.get_mut(player_id) {
            // Allora questo e' perche' se per qualche motivo una sessione
            // che si e' disconnessa e' gia stata "reclaimed" da qualcuno
            // Arc ptr diventa diverso, quindi semplicemente non disconnettere (race cond)
            if !Arc::ptr_eq(&player.sender, our_sender) {
                info!(
                    "[disconnect] '{}' old connection cleaned up (session reclaimed by new connection)",
                    player.username
                );
                return;
            }
            player.connected = false;
            (player.username.clone(), player.room_id.clone())
        } else {
            return;
        }
    };

    if let Some(rid) = &room_id {
        let all_disconnected = {
            if let Some(mut room) = state.rooms.get_mut(rid) {
                room.set_connected(player_id, false);
                room.all_disconnected()
            } else {
                return;
            }
        };

        if all_disconnected {
            info!("[cleanup] all players disconnected from room {} -- removing", &rid[..8]);
            if let Some((_, room)) = state.rooms.remove(rid) {
                for slot in &room.players {
                    state.players.remove(&slot.player_id);
                }
            }
        } else {
            info!("[disconnect] '{}' marked disconnected in room {} (session preserved)", username, &rid[..8]);
            broadcast_to_room(
                state,
                rid,
                &ServerMessage::PlayerDisconnected {
                    username: username.clone(),
                },
            )
            .await;

            if let Some(room) = state.rooms.get(rid) {
                broadcast_to_room(
                    state,
                    rid,
                    &ServerMessage::RoomUpdate {
                        room: room.to_room_info(),
                    },
                )
                .await;
            }
        }
    } else {
        info!("[cleanup] '{}' removed (was not in a room)", username);
        state.players.remove(player_id);
    }
}

fn get_username(state: &Arc<ServerState>, player_id: &str) -> String {
    state
        .players
        .get(player_id)
        .map(|p| p.username.clone())
        .unwrap_or_default()
}

fn msg_type_of(msg: &ServerMessage) -> &'static str {
    match msg {
        ServerMessage::AuthResult { .. } => "AuthResult",
        ServerMessage::RoomList { .. } => "RoomList",
        ServerMessage::PlayerList { .. } => "PlayerList",
        ServerMessage::RoomCreated { .. } => "RoomCreated",
        ServerMessage::PlayerJoined { .. } => "PlayerJoined",
        ServerMessage::PlayerLeft { .. } => "PlayerLeft",
        ServerMessage::PlayerConnected { .. } => "PlayerConnected",
        ServerMessage::PlayerDisconnected { .. } => "PlayerDisconnected",
        ServerMessage::ReadyStateChanged { .. } => "ReadyStateChanged",
        ServerMessage::RoomUpdate { .. } => "RoomUpdate",
        ServerMessage::GameStarted { .. } => "GameStarted",
        ServerMessage::StateUpdate { .. } => "StateUpdate",
        ServerMessage::TurnChanged { .. } => "TurnChanged",
        ServerMessage::Error { .. } => "Error",
    }
}

fn client_msg_type(msg: &ClientMessage) -> &'static str {
    match msg {
        ClientMessage::Authenticate { .. } => "Authenticate",
        ClientMessage::ListRooms => "ListRooms",
        ClientMessage::ListPlayers => "ListPlayers",
        ClientMessage::CreateRoom { .. } => "CreateRoom",
        ClientMessage::JoinRoom { .. } => "JoinRoom",
        ClientMessage::LeaveRoom => "LeaveRoom",
        ClientMessage::SetReady { .. } => "SetReady",
        ClientMessage::StartGame => "StartGame",
        ClientMessage::BroadcastState { .. } => "BroadcastState",
        ClientMessage::TurnChange { .. } => "TurnChange",
    }
}
