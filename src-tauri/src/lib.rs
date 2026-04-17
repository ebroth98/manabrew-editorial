mod ai_agent;
mod card_db;
mod commands;
mod game_manager;
mod multiplayer_controller;
mod network;
mod preset_decks;
mod server_client;
mod server_commands;
mod tauri_transport;

use std::path::PathBuf;

use forge_engine_core::game::TypeRegistry;
use game_manager::GameManager;
use server_client::ServerClient;

fn load_type_registry() -> Result<(), String> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "Could not resolve project root".to_string())?
        .to_path_buf();
    let type_lists_path = project_root
        .join("forge")
        .join("forge-gui")
        .join("res")
        .join("lists")
        .join("TypeLists.txt");

    let contents = std::fs::read_to_string(&type_lists_path).map_err(|err| {
        format!(
            "Failed to read TypeLists.txt at {}: {}",
            type_lists_path.display(),
            err
        )
    })?;
    TypeRegistry::load(&contents);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .setup(|_app| {
            load_type_registry().map_err(|err| {
                eprintln!("[startup] {}", err);
                Box::<dyn std::error::Error>::from(err)
            })?;
            Ok(())
        })
        .manage(GameManager::new())
        .manage(ServerClient::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_game,
            commands::respond,
            commands::end_game,
            commands::restore_snapshot,
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
            server_commands::server_set_deck_selection,
            server_commands::server_start_game,
            server_commands::server_broadcast_state,
            server_commands::server_send_room_message,
            server_commands::server_respond,
            commands::start_multiplayer_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
