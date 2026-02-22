use std::sync::Arc;

use crate::error::ServerError;
use crate::protocol::{RoomInfo, RoomStatus};
use crate::room::Room;
use crate::state::ServerState;

pub fn create_room_sync(
    state: &Arc<ServerState>,
    player_id: &str,
    room_name: String,
    max_players: u8,
) -> Result<RoomInfo, ServerError> {
    {
        if let Some(player) = state.players.get(player_id) {
            if let Some(rid) = &player.room_id {
                return Err(ServerError::AlreadyInRoom(rid.clone()));
            }
        } else {
            return Err(ServerError::AuthFailed("Player not found".into()));
        }
    }

    if state.rooms.len() >= state.max_rooms {
        return Err(ServerError::RoomFull("Server room limit reached".into()));
    }

    let username = state
        .players
        .get(player_id)
        .map(|p| p.username.clone())
        .unwrap_or_default();

    let room_id = uuid::Uuid::new_v4().to_string();
    let room = Room::new(
        room_id.clone(),
        room_name,
        player_id.to_string(),
        username,
        max_players,
    );
    let info = room.to_room_info();

    state.rooms.insert(room_id.clone(), room);

    if let Some(mut player) = state.players.get_mut(player_id) {
        player.room_id = Some(room_id);
    }

    Ok(info)
}

pub fn join_room_sync(
    state: &Arc<ServerState>,
    player_id: &str,
    room_id: &str,
) -> Result<RoomInfo, ServerError> {
    {
        if let Some(player) = state.players.get(player_id) {
            if let Some(rid) = &player.room_id {
                return Err(ServerError::AlreadyInRoom(rid.clone()));
            }
        } else {
            return Err(ServerError::AuthFailed("Player not found".into()));
        }
    }

    let username = state
        .players
        .get(player_id)
        .map(|p| p.username.clone())
        .unwrap_or_default();

    let info = {
        let mut room = state
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| ServerError::RoomNotFound(room_id.to_string()))?;

        if room.status != RoomStatus::Lobby {
            return Err(ServerError::GameAlreadyStarted);
        }

        room.add_player(player_id.to_string(), username)
            .map_err(|msg| {
                if msg.contains("full") {
                    ServerError::RoomFull(room_id.to_string())
                } else {
                    ServerError::AlreadyInRoom(room_id.to_string())
                }
            })?;

        room.to_room_info()
    };

    if let Some(mut player) = state.players.get_mut(player_id) {
        player.room_id = Some(room_id.to_string());
    }

    Ok(info)
}

pub fn leave_room_sync(
    state: &Arc<ServerState>,
    player_id: &str,
) -> Result<(), ServerError> {
    let room_id = {
        state
            .players
            .get(player_id)
            .and_then(|p| p.room_id.clone())
            .ok_or(ServerError::NotInRoom)?
    };

    let room_empty = {
        let mut room = state
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| ServerError::RoomNotFound(room_id.clone()))?;

        room.remove_player(player_id);
        room.players.is_empty()
    };

    if room_empty {
        state.rooms.remove(&room_id);
    }

    if let Some(mut player) = state.players.get_mut(player_id) {
        player.room_id = None;
    }

    Ok(())
}

pub fn set_ready_sync(
    state: &Arc<ServerState>,
    player_id: &str,
    ready: bool,
) -> Result<String, ServerError> {
    let room_id = {
        state
            .players
            .get(player_id)
            .and_then(|p| p.room_id.clone())
            .ok_or(ServerError::NotInRoom)?
    };

    {
        let mut room = state
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| ServerError::RoomNotFound(room_id.clone()))?;

        if room.status != RoomStatus::Lobby {
            return Err(ServerError::GameAlreadyStarted);
        }

        room.set_ready(player_id, ready)
            .map_err(|_| ServerError::NotInRoom)?;
    }

    Ok(room_id)
}

pub fn start_game_sync(
    state: &Arc<ServerState>,
    player_id: &str,
) -> Result<(String, Vec<String>), ServerError> {
    let room_id = {
        state
            .players
            .get(player_id)
            .and_then(|p| p.room_id.clone())
            .ok_or(ServerError::NotInRoom)?
    };

    let player_order = {
        let mut room = state
            .rooms
            .get_mut(&room_id)
            .ok_or_else(|| ServerError::RoomNotFound(room_id.clone()))?;

        if !room.is_host(player_id) {
            return Err(ServerError::NotHost);
        }

        if room.status != RoomStatus::Lobby {
            return Err(ServerError::GameAlreadyStarted);
        }

        if !room.all_ready() {
            return Err(ServerError::PlayersNotReady);
        }

        room.status = RoomStatus::InGame;
        room.player_usernames()
    };

    Ok((room_id, player_order))
}
