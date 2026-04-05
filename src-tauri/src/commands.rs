use tauri::{AppHandle, State};

use crate::game_manager::GameManager;
use crate::multiplayer_controller::relay_response;
use crate::preset_decks::PresetDeckInfo;
use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};
use crate::server_client::ServerClient;

#[tauri::command]
pub async fn start_game(
    app: AppHandle,
    gm: State<'_, GameManager>,
    deck_list: Vec<crate::preset_decks::CardIdentity>,
    starting_life: i32,
    commander_name: Option<String>,
    opponent_deck_list: Option<Vec<crate::preset_decks::CardIdentity>>,
) -> Result<String, String> {
    gm.start_game(
        app,
        deck_list,
        starting_life,
        commander_name,
        opponent_deck_list,
    )
}

#[tauri::command]
pub async fn respond(
    app: AppHandle,
    gm: State<'_, GameManager>,
    client: State<'_, ServerClient>,
    action: PlayerAction,
    player_slot: Option<String>,
) -> Result<(), String> {
    match gm.respond(app, action.clone()) {
        Ok(()) => Ok(()),
        Err(e) if e == "No active game session" => {
            let slot =
                player_slot.ok_or_else(|| "Missing player slot for relay response".to_string())?;
            relay_response(&client, &slot, action)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn end_game(gm: State<'_, GameManager>) -> Result<(), String> {
    gm.end_game()
}

#[tauri::command]
pub async fn restore_snapshot(
    gm: State<'_, GameManager>,
    checkpoint_id: u64,
) -> Result<(), String> {
    gm.restore_snapshot(checkpoint_id)
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
    commander_names: Vec<Option<String>>,
    engine_player_index: usize,
    local_is_host: bool,
    starting_life: i32,
) -> Result<String, String> {
    gm.start_multiplayer_game(
        app,
        player_names,
        deck_lists,
        commander_names,
        engine_player_index,
        local_is_host,
        starting_life,
    )
}
