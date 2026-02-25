use tauri::{AppHandle, State};

use crate::game_manager::GameManager;
use crate::preset_decks::PresetDeckInfo;
use crate::prompt::{AgentPrompt, PlayerAction};

#[tauri::command]
pub async fn start_game(
    app: AppHandle,
    gm: State<'_, GameManager>,
    deck_list: Vec<crate::preset_decks::CardIdentity>,
    starting_life: i32,
    commander_name: Option<String>,
) -> Result<String, String> {
    gm.start_game(app, deck_list, starting_life, commander_name)
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

#[tauri::command]
pub fn get_preset_decks() -> Vec<PresetDeckInfo> {
    crate::preset_decks::list_preset_decks()
}

#[tauri::command]
pub async fn start_multiplayer_game(
    app: AppHandle,
    gm: State<'_, GameManager>,
    player_names: Vec<String>,
    deck_lists: Vec<Vec<crate::preset_decks::CardIdentity>>,
    host_player_index: usize,
    starting_life: i32,
) -> Result<String, String> {
    gm.start_multiplayer_game(
        app,
        player_names,
        deck_lists,
        host_player_index,
        starting_life,
    )
}
