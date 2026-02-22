use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Authenticate {
        username: String,
        password: String,
    },

    ListRooms,

    ListPlayers,

    CreateRoom {
        room_name: String,
        max_players: u8,
    },

    JoinRoom {
        room_id: String,
    },

    LeaveRoom,

    SetReady {
        ready: bool,
    },

    StartGame,

    BroadcastState {
        state: serde_json::Value,
    },

    TurnChange {
        new_active_player: String,
        turn_number: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    AuthResult {
        success: bool,
        player_id: Option<String>,
        reconnected: Option<bool>,
        error: Option<String>,
    },

    RoomList {
        rooms: Vec<RoomInfo>,
    },

    PlayerList {
        players: Vec<PlayerInfo>,
    },

    RoomCreated {
        room_id: String,
        room_name: String,
    },

    PlayerJoined {
        room_id: String,
        username: String,
    },

    PlayerLeft {
        room_id: String,
        username: String,
    },

    PlayerConnected {
        username: String,
    },

    PlayerDisconnected {
        username: String,
    },

    ReadyStateChanged {
        username: String,
        ready: bool,
    },

    RoomUpdate {
        room: RoomInfo,
    },

    GameStarted {
        room_id: String,
        player_order: Vec<String>,
    },

    StateUpdate {
        from_player: String,
        state: serde_json::Value,
    },

    TurnChanged {
        from_player: String,
        new_active_player: String,
        turn_number: u32,
    },

    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub room_name: String,
    pub host: String,
    pub players: Vec<RoomPlayerInfo>,
    pub max_players: u8,
    pub status: RoomStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPlayerInfo {
    pub username: String,
    pub ready: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub username: String,
    pub player_id: String,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomStatus {
    Lobby,
    InGame,
}
