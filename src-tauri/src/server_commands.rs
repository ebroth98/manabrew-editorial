use tauri::{AppHandle, State};

use crate::multiplayer_controller::relay_response_value;
use crate::preset_decks::CardIdentity;
use crate::server_client::ServerClient;

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
pub async fn server_disconnect(client: State<'_, ServerClient>) -> Result<(), String> {
    client.disconnect();
    Ok(())
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
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "CreateRoom",
            "room_name": room_name,
            "max_players": max_players,
            "format": format,
        }),
    )
}

#[tauri::command]
pub async fn server_join_room(
    client: State<'_, ServerClient>,
    room_id: String,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "JoinRoom",
            "room_id": room_id,
        }),
    )
}

#[tauri::command]
pub async fn server_leave_room(client: State<'_, ServerClient>) -> Result<(), String> {
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
    deck_list: Vec<CardIdentity>,
    commander_name: Option<String>,
) -> Result<(), String> {
    send_server_message(
        &client,
        serde_json::json!({
            "type": "SetDeckSelection",
            "deck_name": deck_name,
            "deck_list": deck_list,
            "commander_name": commander_name,
        }),
    )
}

#[tauri::command]
pub async fn server_start_game(client: State<'_, ServerClient>) -> Result<(), String> {
    send_server_message(&client, serde_json::json!({"type": "StartGame"}))
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
