use tauri::{AppHandle, State};

use crate::client_bot::{ClientBotConfig, ClientBotManager};
use crate::multiplayer_controller::relay_response_value;
use crate::server_client::ServerClient;
use forge_agent_interface::deck_dto::Deck;

fn send_server_message(client: &ServerClient, msg: serde_json::Value) -> Result<(), String> {
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_connect(
    app: AppHandle,
    client: State<'_, ServerClient>,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), String> {
    client.connect(app, host, port, username, password)
}

#[tauri::command]
pub async fn server_disconnect(
    client: State<'_, ServerClient>,
    bot_manager: State<'_, ClientBotManager>,
) -> Result<(), String> {
    bot_manager.stop_all();
    client.disconnect();
    Ok(())
}

#[tauri::command]
pub async fn server_spawn_ai_bot(
    client: State<'_, ServerClient>,
    bot_manager: State<'_, ClientBotManager>,
    room_id: String,
    username: String,
    deck_name: String,
    deck: Deck,
    commander_name: Option<String>,
) -> Result<(), String> {
    let connection = client.connection_config()?;
    bot_manager.spawn_bot(ClientBotConfig {
        connection,
        room_id,
        username,
        deck_name,
        deck,
        commander_name,
    })
}

#[tauri::command]
pub async fn server_remove_ai_bot(
    bot_manager: State<'_, ClientBotManager>,
    username: String,
) -> Result<(), String> {
    if bot_manager.stop_bot(&username) {
        Ok(())
    } else {
        Err(format!("No bot found with username '{}'", username))
    }
}

#[tauri::command]
pub async fn server_list_rooms(client: State<'_, ServerClient>) -> Result<(), String> {
    send_server_message(&client, serde_json::json!({"type": "ListRooms"}))
}

#[tauri::command]
pub async fn server_list_players(client: State<'_, ServerClient>) -> Result<(), String> {
    send_server_message(&client, serde_json::json!({"type": "ListPlayers"}))
}

#[tauri::command]
pub async fn server_create_room(
    client: State<'_, ServerClient>,
    room_name: String,
    max_players: u8,
    format: String,
    hosted: Option<bool>,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "CreateRoom",
            "room_name": room_name,
            "max_players": max_players,
            "format": format,
            "hosted": hosted.unwrap_or(false),
        }),
    )
}

#[tauri::command]
pub async fn server_join_room(
    client: State<'_, ServerClient>,
    room_id: String,
    observe: Option<bool>,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "JoinRoom",
            "room_id": room_id,
            "observe": observe.unwrap_or(false),
        }),
    )
}

#[tauri::command]
pub async fn server_leave_room(
    client: State<'_, ServerClient>,
    bot_manager: State<'_, ClientBotManager>,
) -> Result<(), String> {
    bot_manager.stop_all();
    send_server_message(&client, serde_json::json!({"type": "LeaveRoom"}))
}

#[tauri::command]
pub async fn server_set_ready(client: State<'_, ServerClient>, ready: bool) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "SetReady",
            "ready": ready,
        }),
    )
}

#[tauri::command]
pub async fn server_set_deck_selection(
    client: State<'_, ServerClient>,
    deck_name: String,
    deck: Deck,
    commander_name: Option<String>,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "SetDeckSelection",
            "deck_name": deck_name,
            "deck": deck,
            "commander_name": commander_name,
        }),
    )
}

#[tauri::command]
pub async fn server_start_game(client: State<'_, ServerClient>) -> Result<(), String> {
    send_server_message(&client, serde_json::json!({"type": "StartGame"}))
}

#[tauri::command]
pub async fn server_broadcast_state(
    client: State<'_, ServerClient>,
    state: serde_json::Value,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "BroadcastState",
            "state": state,
        }),
    )
}

#[tauri::command]
pub async fn server_send_room_message(
    client: State<'_, ServerClient>,
    message: serde_json::Value,
) -> Result<(), String> {
    if message.get("kind").and_then(|value| value.as_str()) != Some("roomRelay") {
        return Err("Room message must be a roomRelay envelope".to_string());
    }
    send_server_message(
        &client,
        serde_json::json!({
            "type": "BroadcastState",
            "state": message,
        }),
    )
}

/// Remote player sends their response back to the host via the server relay.
#[tauri::command]
pub async fn server_respond(
    client: State<'_, ServerClient>,
    player_slot: String,
    action: serde_json::Value,
) -> Result<(), String> {
    relay_response_value(&client, &player_slot, action)
}
