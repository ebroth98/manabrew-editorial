use tauri::{AppHandle, State};

use crate::game_manager::GameManager;
use crate::prompt::{AgentPrompt, PlayerAction};

#[tauri::command]
pub async fn start_game(
    app: AppHandle,
    gm: State<'_, GameManager>,
    deck_choice: String,
) -> Result<String, String> {
    gm.start_game(app, &deck_choice)
}

#[tauri::command]
pub async fn respond(
    app: AppHandle,
    gm: State<'_, GameManager>,
    action: PlayerAction,
) -> Result<(), String> {
    gm.respond(app, action)
}

#[tauri::command]
pub async fn end_game(gm: State<'_, GameManager>) -> Result<(), String> {
    gm.end_game()
}

#[tauri::command]
pub async fn get_prompt(gm: State<'_, GameManager>) -> Result<Option<AgentPrompt>, String> {
    Ok(gm.get_latest_prompt())
}
