use serde::{Deserialize, Serialize};

use crate::game_view_dto::{GameViewDto, CardDto};

/// A display-only event that the frontend should animate before rendering the prompt's game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DisplayEvent {
    #[serde(rename_all = "camelCase")]
    CardPlayed {
        card_id: String,
        card_name: String,
        player_id: String,
    },
    #[serde(rename_all = "camelCase")]
    TurnChanged {
        active_player_id: String,
        active_player_name: String,
        turn_number: u32,
    },
}

/// Sent from game thread to frontend: what the human player must decide,
/// bundled with any display events that happened since the last prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPrompt {
    /// Display events to animate before applying the game state.
    #[serde(default)]
    pub display_events: Vec<DisplayEvent>,
    /// The actual prompt data (type + gameView + prompt-specific fields).
    #[serde(flatten)]
    pub inner: AgentPromptInner,
}

/// The actual decision prompt variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentPromptInner {
    Mulligan {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "handCardIds")]
        hand_card_ids: Vec<String>,
    },
    ChooseAction {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "playableCardIds")]
        playable_card_ids: Vec<String>,
        /// Untapped lands on the battlefield that the player can manually tap for mana.
        #[serde(rename = "tappableLandIds")]
        tappable_land_ids: Vec<String>,
        /// Tapped lands whose mana is still in the pool (can be untapped to undo).
        #[serde(rename = "untappableLandIds")]
        untappable_land_ids: Vec<String>,
    },
    ChooseAttackers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "availableAttackerIds")]
        available_attacker_ids: Vec<String>,
    },
    ChooseBlockers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerIds")]
        attacker_ids: Vec<String>,
        #[serde(rename = "availableBlockerIds")]
        available_blocker_ids: Vec<String>,
    },
    ChooseTargetPlayer {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validPlayerIds")]
        valid_player_ids: Vec<String>,
    },
    ChooseTargetCard {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
    },
    ChooseTargetAny {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validPlayerIds")]
        valid_player_ids: Vec<String>,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
    },
    ChooseTargetCardFromZone {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        zone: String,
        #[serde(rename = "zoneCards")]
        zone_cards: Vec<CardDto>,
    },
    GameOver {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
    },
    /// Display-only state update — no player decision required.
    /// Emitted after each card play / turn change so the frontend can
    /// animate events one-at-a-time even during the AI's turn.
    StateUpdate {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
    },
    /// Scry N: player sees `card_ids` (top N of library) and picks which go to bottom.
    Scry {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// The top N cards the player is looking at (in library order, last = topmost).
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        /// Card DTOs for display.
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
    },
    /// Surveil N: player sees `card_ids` (top N of library) and picks which go to graveyard.
    Surveil {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
    },
    /// Dig N, take K: player sees `card_ids` (top N) and picks up to `num_to_take` to keep.
    Dig {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
        #[serde(rename = "numToTake")]
        num_to_take: usize,
        optional: bool,
    },
}

/// Sent from frontend to game thread: the human player's response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PlayerAction {
    MulliganDecision {
        keep: bool,
    },
    PlayCard {
        #[serde(rename = "cardId")]
        card_id: Option<String>,
    },
    DeclareAttackers {
        #[serde(rename = "attackerIds")]
        attacker_ids: Vec<String>,
    },
    DeclareBlockers {
        assignments: Vec<BlockAssignment>,
    },
    TargetPlayer {
        #[serde(rename = "playerId")]
        player_id: Option<String>,
    },
    TargetCard {
        #[serde(rename = "cardId")]
        card_id: Option<String>,
    },
    TargetAny {
        target: TargetAnyChoice,
    },
    TapLand {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    UntapLand {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    /// Response to Scry prompt: IDs of cards the player wants on the bottom.
    ScryDecision {
        #[serde(rename = "bottomCardIds")]
        bottom_card_ids: Vec<String>,
    },
    /// Response to Surveil prompt: IDs of cards the player wants in the graveyard.
    SurveilDecision {
        #[serde(rename = "graveyardCardIds")]
        graveyard_card_ids: Vec<String>,
    },
    /// Response to Dig prompt: IDs of the cards the player wants to take.
    DigDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    Concede,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAssignment {
    pub blocker_id: String,
    pub attacker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TargetAnyChoice {
    Player {
        #[serde(rename = "playerId")]
        player_id: String,
    },
    Card {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    None,
}
