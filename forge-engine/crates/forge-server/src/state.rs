use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::room::Room;

/// Sender half of a WebSocket connection.
pub type WsSender = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    tokio_tungstenite::tungstenite::Message,
>;

pub struct ConnectedPlayer {
    pub player_id: String,
    pub username: String,
    pub room_id: Option<String>,
    pub sender: Arc<Mutex<WsSender>>,
    pub connected: bool,
}

pub struct ServerState {
    pub players: DashMap<String, ConnectedPlayer>,
    pub rooms: DashMap<String, Room>,
    pub server_key: String,
    pub max_rooms: usize,
}

impl ServerState {
    pub fn new(server_key: String, max_rooms: usize) -> Self {
        ServerState {
            players: DashMap::new(),
            rooms: DashMap::new(),
            server_key,
            max_rooms,
        }
    }

    pub fn username_taken_by_connected(&self, username: &str) -> bool {
        self.players
            .iter()
            .any(|entry| entry.value().username == username && entry.value().connected)
    }

    pub fn find_disconnected_by_username(&self, username: &str) -> Option<(String, Option<String>)> {
        self.players
            .iter()
            .find(|entry| entry.value().username == username && !entry.value().connected)
            .map(|entry| (entry.value().player_id.clone(), entry.value().room_id.clone()))
    }
}
