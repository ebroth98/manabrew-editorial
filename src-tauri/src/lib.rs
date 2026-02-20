mod ai_agent;
mod commands;
mod game_manager;
mod game_view_dto;
mod prompt;
mod tauri_agent;

use game_manager::GameManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(GameManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::start_game,
            commands::respond,
            commands::end_game,
            commands::get_prompt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
