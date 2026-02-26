use serde_json::Value;

use crate::ids_codec::parse_player_slot;
use crate::prompt::{AgentPrompt, PlayerAction};

pub enum RelayEnvelope {
    Prompt {
        for_player: String,
        prompt: AgentPrompt,
    },
    Response {
        from_player: String,
        action: PlayerAction,
    },
}

pub fn encode_relay_envelope(envelope: RelayEnvelope) -> Result<Value, String> {
    match envelope {
        RelayEnvelope::Prompt { for_player, prompt } => Ok(serde_json::json!({
            "kind": "prompt",
            "forPlayer": for_player,
            "prompt": prompt,
        })),
        RelayEnvelope::Response {
            from_player,
            action,
        } => {
            let action_value = serde_json::to_value(action).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "kind": "response",
                "fromPlayer": from_player,
                "action": action_value,
            }))
        }
    }
}

pub fn wrap_broadcast_state(state: Value) -> String {
    serde_json::json!({
        "type": "BroadcastState",
        "state": state,
    })
    .to_string()
}

pub fn decode_relay_response(state: &Value) -> Result<(usize, PlayerAction), String> {
    let kind = state
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing kind in relay envelope".to_string())?;
    if kind != "response" {
        return Err(format!("Unsupported relay kind: {}", kind));
    }

    let from_player = state
        .get("fromPlayer")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing fromPlayer in response envelope".to_string())?;
    let player_index =
        parse_player_slot(from_player).ok_or_else(|| format!("Invalid fromPlayer: {}", from_player))?;

    let action_value = state
        .get("action")
        .ok_or_else(|| "Missing action in response envelope".to_string())?;
    let action: PlayerAction =
        serde_json::from_value(action_value.clone()).map_err(|e| format!("Invalid action: {}", e))?;

    Ok((player_index, action))
}
