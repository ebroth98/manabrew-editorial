use tauri::{AppHandle, State};

use crate::server_client::ServerClient;

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
    let msg = serde_json::json!({"type": "ListRooms"});
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_list_players(client: State<'_, ServerClient>) -> Result<(), String> {
    let msg = serde_json::json!({"type": "ListPlayers"});
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_create_room(
    client: State<'_, ServerClient>,
    room_name: String,
    max_players: u8,
    format: String,
) -> Result<(), String> {
    let msg = serde_json::json!({
        "type": "CreateRoom",
        "room_name": room_name,
        "max_players": max_players,
        "format": format,
    });
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_join_room(
    client: State<'_, ServerClient>,
    room_id: String,
) -> Result<(), String> {
    let msg = serde_json::json!({
        "type": "JoinRoom",
        "room_id": room_id,
    });
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_leave_room(client: State<'_, ServerClient>) -> Result<(), String> {
    let msg = serde_json::json!({"type": "LeaveRoom"});
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_set_ready(
    client: State<'_, ServerClient>,
    ready: bool,
) -> Result<(), String> {
    let msg = serde_json::json!({
        "type": "SetReady",
        "ready": ready,
    });
    client.send(&msg.to_string())
}

#[tauri::command]
pub async fn server_start_game(client: State<'_, ServerClient>) -> Result<(), String> {
    let msg = serde_json::json!({"type": "StartGame"});
    client.send(&msg.to_string())
}
