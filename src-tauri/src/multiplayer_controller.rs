use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tauri::{AppHandle, Emitter, Manager};

use crate::game_log_event::GameLogEntryDto;
use crate::game_snapshot_event::GameSnapshotEventDto;
use crate::ids_codec::player_slot;
use crate::network::{
    decode_relay_response, encode_relay_envelope, wrap_broadcast_state, RelayEnvelope,
};
use crate::prompt::{AgentPrompt, PlayerAction};
use crate::server_client::ServerClient;

pub fn spawn_engine_prompt_forwarder(
    app: AppHandle,
    latest_prompt: Arc<Mutex<Option<AgentPrompt>>>,
    rx: mpsc::Receiver<AgentPrompt>,
) {
    thread::spawn(move || {
        eprintln!("[prompt_fwd] Engine prompt forwarder started");
        while let Ok(prompt) = rx.recv() {
            if let Ok(mut lp) = latest_prompt.lock() {
                *lp = Some(prompt.clone());
            }
            let _ = app.emit("game:prompt", &prompt);
        }
        eprintln!("[prompt_fwd] Engine prompt forwarder ended");
    });
}

pub fn spawn_notify_forwarder(
    app: AppHandle,
    rx: mpsc::Receiver<GameLogEntryDto>,
    relay_from_player: Option<String>,
) {
    thread::spawn(move || {
        let window = app.get_webview_window("main");
        while let Ok(msg) = rx.recv() {
            let _ = if let Some(ref w) = window {
                w.emit("game:log", &msg)
            } else {
                app.emit("game:log", &msg)
            };
            if let Some(from_player) = relay_from_player.as_ref() {
                let envelope = match encode_relay_envelope(RelayEnvelope::Log {
                    from_player: from_player.clone(),
                    entry: msg,
                }) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[log_fwd] Failed to encode log envelope: {}", e);
                        continue;
                    }
                };
                let payload = wrap_broadcast_state(envelope);
                if let Some(client) = app.try_state::<ServerClient>() {
                    let _ = client.send(&payload);
                }
            }
        }
    });
}

pub fn spawn_snapshot_forwarder(
    app: AppHandle,
    rx: mpsc::Receiver<GameSnapshotEventDto>,
    relay_from_player: Option<String>,
) {
    thread::spawn(move || {
        let window = app.get_webview_window("main");
        while let Ok(msg) = rx.recv() {
            let _ = if let Some(ref w) = window {
                w.emit("game:snapshot", &msg)
            } else {
                app.emit("game:snapshot", &msg)
            };
            if let Some(from_player) = relay_from_player.as_ref() {
                let envelope = match encode_relay_envelope(RelayEnvelope::Snapshot {
                    from_player: from_player.clone(),
                    entry: msg,
                }) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[snapshot_fwd] Failed to encode snapshot envelope: {}", e);
                        continue;
                    }
                };
                let payload = wrap_broadcast_state(envelope);
                if let Some(client) = app.try_state::<ServerClient>() {
                    let _ = client.send(&payload);
                }
            }
        }
    });
}

pub fn spawn_remote_prompt_forwarder(app: AppHandle, rx: mpsc::Receiver<(usize, AgentPrompt)>) {
    thread::spawn(move || {
        eprintln!("[remote_fwd] Remote prompt forwarder started");
        while let Ok((player_index, prompt)) = rx.recv() {
            let for_player = player_slot(player_index);
            let envelope = match encode_relay_envelope(RelayEnvelope::Prompt { for_player, prompt })
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[remote_fwd] Failed to encode prompt envelope: {}", e);
                    continue;
                }
            };
            let msg = wrap_broadcast_state(envelope);
            if let Some(client) = app.try_state::<ServerClient>() {
                let _ = client.send(&msg);
            }
        }
        eprintln!("[remote_fwd] Remote prompt forwarder ended");
    });
}

pub fn relay_response(
    client: &ServerClient,
    player_slot: &str,
    action: PlayerAction,
) -> Result<(), String> {
    let envelope = encode_relay_envelope(RelayEnvelope::Response {
        from_player: player_slot.to_string(),
        action,
    })?;
    let msg = wrap_broadcast_state(envelope);
    client.send(&msg)
}

pub fn relay_response_value(
    client: &ServerClient,
    player_slot: &str,
    action: serde_json::Value,
) -> Result<(), String> {
    let envelope = serde_json::json!({
        "kind": "response",
        "fromPlayer": player_slot,
        "action": action,
    });
    let msg = wrap_broadcast_state(envelope);
    client.send(&msg)
}

pub fn parse_remote_response(state: &serde_json::Value) -> Result<(usize, PlayerAction), String> {
    decode_relay_response(state)
}
