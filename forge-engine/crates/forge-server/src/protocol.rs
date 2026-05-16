pub use forge_agent_interface::deck_dto::Deck;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDeckInfo {
    pub username: String,
    pub deck_name: String,
    pub deck: Deck,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commander_name: Option<String>,
}

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
        format: GameFormat,
        #[serde(default)]
        hosted: bool,
    },

    JoinRoom {
        room_id: String,
        #[serde(default)]
        observe: bool,
    },

    LeaveRoom,

    SetReady {
        ready: bool,
    },

    SetDeckSelection {
        deck_name: String,
        deck: Deck,
        commander_name: Option<String>,
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
        player_decks: Vec<PlayerDeckInfo>,
        starting_life: i32,
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
    #[serde(default)]
    pub hosted: bool,
    pub players: Vec<RoomPlayerInfo>,
    pub max_players: u8,
    pub format: GameFormat,
    pub status: RoomStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPlayerInfo {
    pub username: String,
    pub ready: bool,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_deck_name: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameFormat {
    Standard,
    Pioneer,
    Modern,
    Legacy,
    Vintage,
    Pauper,
    Commander,
    Brawl,
    Oathbreaker,
    Draft,
    Sealed,
}
