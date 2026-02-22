mod ai_agent;
mod card_db;
mod commands;
mod game_manager;
mod game_view_dto;
mod preset_decks;
mod prompt;
mod remote_agent;
mod server_client;
mod server_commands;
mod tauri_agent;

use game_manager::GameManager;
use server_client::ServerClient;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(GameManager::new())
        .manage(ServerClient::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_game,
            commands::respond,
            commands::end_game,
            commands::get_prompt,
            commands::get_preset_decks,
            server_commands::server_connect,
            server_commands::server_disconnect,
            server_commands::server_list_rooms,
            server_commands::server_list_players,
            server_commands::server_create_room,
            server_commands::server_join_room,
            server_commands::server_leave_room,
            server_commands::server_set_ready,
            server_commands::server_start_game,
            server_commands::server_respond,
            commands::start_multiplayer_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
