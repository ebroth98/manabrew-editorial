#![allow(clippy::too_many_arguments)]

mod ai_agent;
mod card_db;
mod client_bot;
mod commands;
mod engine_backend;
mod game_manager;
mod multiplayer_controller;
mod network;
mod preset_decks;
mod server_client;
mod server_commands;
mod tauri_transport;

use client_bot::ClientBotManager;
use forge_engine_core::game::TypeRegistry;
use game_manager::GameManager;
use server_client::ServerClient;
use tauri::Manager;

// Embedded at compile time so packaged builds don't depend on the build
// machine's source tree layout.
const TYPE_LISTS: &str = include_str!("../../forge/forge-gui/res/lists/TypeLists.txt");

// Tauri copies these resources into the app's resource_dir at install
// time (see tauri.conf.json bundle.resources). The card/token/edition/
// preset loaders pick them up via env vars set before any command runs.
//
// `CARDSET_ARCHIVE` points at a single bundled file, not a directory —
// without it, packaged builds fall back to scanning the 32k-file
// cardsfolder, which on Windows takes ~10 minutes due to NTFS + Defender
// per-file overhead.
const RESOURCE_ENV_MAP: &[(&str, &str)] = &[
    ("CARDS_DIR", "cardsfolder"),
    ("TOKEN_SCRIPTS_DIR", "tokenscripts"),
    ("EDITIONS_DIR", "editions"),
    ("PRESET_DECKS_DIR", "preset_decks"),
    ("CARDSET_ARCHIVE", "cardset.rkyv"),
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            TypeRegistry::load(TYPE_LISTS);

            if let Ok(resource_dir) = app.path().resource_dir() {
                for (key, subdir) in RESOURCE_ENV_MAP {
                    let path = resource_dir.join(subdir);
                    if path.exists() {
                        std::env::set_var(key, &path);
                    } else {
                        eprintln!(
                            "[startup] bundled resource missing: {} (env {} left unset)",
                            path.display(),
                            key
                        );
                    }
                }
            }

            Ok(())
        })
        .manage(GameManager::new())
        .manage(ServerClient::new())
        .manage(ClientBotManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_game,
            commands::respond,
            commands::end_game,
            commands::restore_snapshot,
            commands::get_prompt,
            commands::get_preset_decks,
            server_commands::server_connect,
            server_commands::server_disconnect,
            server_commands::server_spawn_ai_bot,
            server_commands::server_remove_ai_bot,
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
