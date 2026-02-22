use crate::protocol::{GameFormat, RoomInfo, RoomPlayerInfo, RoomStatus};

#[derive(Debug, Clone)]
pub struct RoomSlot {
    pub player_id: String,
    pub username: String,
    pub ready: bool,
    pub connected: bool,
}

#[derive(Debug)]
pub struct Room {
    pub room_id: String,
    pub room_name: String,
    pub host_player_id: String,
    pub max_players: u8,
    pub format: GameFormat,
    pub status: RoomStatus,
    pub players: Vec<RoomSlot>,
}

impl Room {
    pub fn new(
        room_id: String,
        room_name: String,
        host_player_id: String,
        host_username: String,
        max_players: u8,
        format: GameFormat,
    ) -> Self {
        let max_players = max_players.clamp(2, 8);
        Room {
            room_id,
            room_name,
            host_player_id: host_player_id.clone(),
            max_players,
            format,
            status: RoomStatus::Lobby,
            players: vec![RoomSlot {
                player_id: host_player_id,
                username: host_username,
                ready: false,
                connected: true,
            }],
        }
    }

    pub fn is_full(&self) -> bool {
        self.players.len() >= self.max_players as usize
    }

    pub fn all_ready(&self) -> bool {
        self.players.len() >= 2 && self.players.iter().all(|p| p.ready)
    }

    pub fn add_player(&mut self, player_id: String, username: String) -> Result<(), String> {
        if self.is_full() {
            return Err("Room is full".into());
        }
        if self.status != RoomStatus::Lobby {
            return Err("Game already started".into());
        }
        if self.players.iter().any(|p| p.player_id == player_id) {
            return Err("Already in this room".into());
        }
        self.players.push(RoomSlot {
            player_id,
            username,
            ready: false,
            connected: true,
        });
        Ok(())
    }

    pub fn remove_player(&mut self, player_id: &str) -> Option<RoomSlot> {
        if let Some(idx) = self.players.iter().position(|p| p.player_id == player_id) {
            let slot = self.players.remove(idx);
            // If the host left, promote the first remaining player
            if self.host_player_id == player_id {
                if let Some(new_host) = self.players.first() {
                    self.host_player_id = new_host.player_id.clone();
                }
            }
            Some(slot)
        } else {
            None
        }
    }

    pub fn set_connected(&mut self, player_id: &str, connected: bool) {
        if let Some(slot) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            slot.connected = connected;
        }
    }

    pub fn all_disconnected(&self) -> bool {
        self.players.iter().all(|p| !p.connected)
    }

    pub fn set_ready(&mut self, player_id: &str, ready: bool) -> Result<(), String> {
        if let Some(slot) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            slot.ready = ready;
            Ok(())
        } else {
            Err("Player not in room".into())
        }
    }

    pub fn is_host(&self, player_id: &str) -> bool {
        self.host_player_id == player_id
    }

    pub fn host_username(&self) -> String {
        self.players
            .iter()
            .find(|p| p.player_id == self.host_player_id)
            .map(|p| p.username.clone())
            .unwrap_or_default()
    }

    pub fn player_usernames(&self) -> Vec<String> {
        self.players.iter().map(|p| p.username.clone()).collect()
    }

    pub fn player_ids(&self) -> Vec<String> {
        self.players.iter().map(|p| p.player_id.clone()).collect()
    }

    pub fn connected_player_ids(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|p| p.connected)
            .map(|p| p.player_id.clone())
            .collect()
    }

    pub fn to_room_info(&self) -> RoomInfo {
        RoomInfo {
            room_id: self.room_id.clone(),
            room_name: self.room_name.clone(),
            host: self.host_username(),
            players: self
                .players
                .iter()
                .map(|p| RoomPlayerInfo {
                    username: p.username.clone(),
                    ready: p.ready,
                    connected: p.connected,
                })
                .collect(),
            max_players: self.max_players,
            format: self.format.clone(),
            status: self.status.clone(),
        }
    }
}
