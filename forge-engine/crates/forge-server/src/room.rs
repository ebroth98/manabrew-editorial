use crate::protocol::{
    CardIdentity, GameFormat, PlayerDeckInfo, RoomInfo, RoomPlayerInfo, RoomStatus,
};

#[derive(Debug, Clone)]
pub struct RoomSlot {
    pub player_id: String,
    pub username: String,
    pub ready: bool,
    pub connected: bool,
    pub selected_deck_name: Option<String>,
    pub selected_deck_list: Vec<CardIdentity>,
    pub selected_commander_name: Option<String>,
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
                selected_deck_name: None,
                selected_deck_list: vec![],
                selected_commander_name: None,
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
            selected_deck_name: None,
            selected_deck_list: vec![],
            selected_commander_name: None,
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

    /// Forward-ported for future use when room cleanup logic is implemented.
    #[allow(dead_code)]
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

    pub fn set_deck_selection(
        &mut self,
        player_id: &str,
        deck_name: String,
        deck_list: Vec<CardIdentity>,
        commander_name: Option<String>,
    ) -> Result<(), String> {
        if let Some(slot) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            slot.selected_deck_name = Some(deck_name);
            slot.selected_deck_list = deck_list;
            slot.selected_commander_name = commander_name;
            slot.ready = false;
            Ok(())
        } else {
            Err("Player not in room".into())
        }
    }

    pub fn has_selected_deck(&self, player_id: &str) -> bool {
        self.players
            .iter()
            .find(|p| p.player_id == player_id)
            .map(|p| !p.selected_deck_list.is_empty() && p.selected_deck_name.is_some())
            .unwrap_or(false)
    }

    pub fn player_decks(&self) -> Vec<PlayerDeckInfo> {
        self.players
            .iter()
            .map(|p| PlayerDeckInfo {
                username: p.username.clone(),
                deck_name: p
                    .selected_deck_name
                    .clone()
                    .unwrap_or_else(|| "Unknown Deck".to_string()),
                deck_list: p.selected_deck_list.clone(),
                commander_name: p.selected_commander_name.clone(),
            })
            .collect()
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

    /// Forward-ported for future use when full player ID tracking is needed.
    #[allow(dead_code)]
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
                    selected_deck_name: p.selected_deck_name.clone(),
                })
                .collect(),
            max_players: self.max_players,
            format: self.format.clone(),
            status: self.status.clone(),
        }
    }
}
