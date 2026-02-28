//! Shared JSON protocol types for cross-engine differential testing.
//!
//! All types derive `Serialize` and `Deserialize` so both the Rust engine and
//! a future Java harness can exchange them as JSON over stdout/stdin.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Decision Points ────────────────────────────────────────────────

/// Emitted when a player must make a choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub turn: u32,
    pub phase: String,
    pub active_player: u32,
    pub deciding_player: u32,
    pub decision_type: DecisionType,
    pub options: DecisionOptions,
}

/// The kind of decision being requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    Mulligan,
    MainPhaseAction,
    DeclareAttackers,
    DeclareBlockers,
    ChooseTargetPlayer,
    ChooseTargetCard,
    ChooseTargetAny,
    ChooseSacrifice,
    ChooseDiscard,
    ChooseScry,
    ChooseSurveil,
    ChooseMode,
    ChooseOptionalTrigger,
    ChooseLandOrSpell,
}

/// Available options for a decision, keyed by card/player name (not IDs).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionOptions {
    /// Card names that can be played (main phase)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub playable_cards: Vec<String>,
    /// Land names that can be tapped for mana
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tappable_lands: Vec<String>,
    /// Creature names eligible to attack
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attackers: Vec<String>,
    /// Creature names eligible to block
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub blockers: Vec<String>,
    /// Valid target player indices
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub valid_player_targets: Vec<u32>,
    /// Valid target card names
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub valid_card_targets: Vec<String>,
    /// Card names in hand (for discard decisions)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hand_cards: Vec<String>,
    /// Number of cards to discard
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discard_count: Option<usize>,
    /// Mode descriptions (for modal spells)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mode_descriptions: Vec<String>,
    /// Min/max modes to pick
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_modes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_modes: Option<usize>,
    /// Trigger description (for optional triggers)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trigger_description: Option<String>,
}

/// A decision fed back to the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Decision {
    Mulligan { keep: bool },
    MainPhaseAction { action: String },
    DeclareAttackers { names: Vec<String> },
    DeclareBlockers { pairs: Vec<(String, String)> },
    ChooseTargetPlayer { player_id: u32 },
    ChooseTargetCard { name: String },
    ChooseTargetAny { target: TargetAnyChoice },
    ChooseSacrifice { name: String },
    ChooseDiscard { names: Vec<String> },
    ChooseScry { to_bottom: Vec<String> },
    ChooseSurveil { to_graveyard: Vec<String> },
    ChooseMode { indices: Vec<usize> },
    ChooseOptionalTrigger { accept: bool },
    ChooseLandOrSpell { land: Option<bool> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TargetAnyChoice {
    Player { player_id: u32 },
    Card { name: String },
    None,
}

// ── State Snapshots ────────────────────────────────────────────────

/// Normalized game state for comparison between engines.
/// All lists are sorted alphabetically by card name for deterministic comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSnapshot {
    pub turn: u32,
    pub phase: String,
    pub active_player: u32,
    pub game_over: bool,
    pub winner: Option<u32>,
    pub players: Vec<PlayerSnapshot>,
    pub stack: Vec<String>,
}

/// Per-player state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSnapshot {
    pub name: String,
    pub index: u32,
    pub life: i32,
    pub poison: i32,
    pub lands_played: i32,
    pub has_lost: bool,
    pub has_won: bool,
    pub battlefield: Vec<CardSnapshot>,
    pub graveyard: Vec<String>,
    pub hand: Vec<String>,
    pub exile: Vec<String>,
    pub library_size: usize,
}

/// A card on the battlefield, normalized for comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardSnapshot {
    pub name: String,
    pub tapped: bool,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub damage: i32,
    pub summoning_sick: bool,
    pub counters: BTreeMap<String, i32>,
    pub controller: u32,
}

// ── Game Trace ─────────────────────────────────────────────────────

/// A complete record of a game: per-phase snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTrace {
    pub seed: u64,
    pub deck1: String,
    pub deck2: String,
    pub max_turns: u32,
    pub snapshots: Vec<StateSnapshot>,
}

// ── Parity Report ──────────────────────────────────────────────────

/// A divergence between Rust and Java engine state at a specific phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Divergence {
    pub snapshot_index: usize,
    pub turn: u32,
    pub phase: String,
    pub field: String,
    pub rust_value: String,
    pub java_value: String,
}

/// Full parity report comparing two engine runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityReport {
    pub seed: u64,
    pub deck1: String,
    pub deck2: String,
    pub total_snapshots: usize,
    pub divergences: Vec<Divergence>,
    pub passed: bool,
}

// ── Matrix Mode Types ──────────────────────────────────────────────

/// Status of a single matchup in matrix mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchupStatus {
    Pass,
    Fail,
    Error,
}

/// Result of a single deck-pair + seed matchup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchupResult {
    pub deck1: String,
    pub deck2: String,
    pub seed: u64,
    pub status: MatchupStatus,
    pub snapshots_compared: usize,
    pub divergence_count: usize,
    pub first_divergence: Option<Divergence>,
    pub error_message: Option<String>,
}

/// Aggregate report for all matchups in matrix mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixReport {
    pub total_matchups: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub seeds: Vec<u64>,
    pub decks: Vec<String>,
    pub max_turns: u32,
    pub results: Vec<MatchupResult>,
}

// ── Fuzz Mode Types ──────────────────────────────────────────────

/// Result of a single fuzz iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResult {
    pub iteration: usize,
    pub game_seed: u64,
    /// Inline-format deck spec for player 1 (for reproducibility).
    pub deck1_spec: String,
    /// Inline-format deck spec for player 2 (for reproducibility).
    pub deck2_spec: String,
    pub result: MatchupResult,
}

/// Aggregate report for fuzz random deck testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzReport {
    pub master_seed: u64,
    pub iterations: usize,
    pub max_turns: u32,
    /// Number of cards in the discovered pool.
    pub pool_size: usize,
    /// Total cards in the database.
    pub total_cards: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub results: Vec<FuzzResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_roundtrip_json() {
        let snap = StateSnapshot {
            turn: 3,
            phase: "Main1".into(),
            active_player: 0,
            game_over: false,
            winner: None,
            players: vec![PlayerSnapshot {
                name: "Alice".into(),
                index: 0,
                life: 20,
                poison: 0,
                lands_played: 1,
                has_lost: false,
                has_won: false,
                battlefield: vec![CardSnapshot {
                    name: "Mountain".into(),
                    tapped: true,
                    power: None,
                    toughness: None,
                    damage: 0,
                    summoning_sick: false,
                    counters: BTreeMap::new(),
                    controller: 0,
                }],
                graveyard: vec![],
                hand: vec!["Lightning Bolt".into()],
                exile: vec![],
                library_size: 30,
            }],
            stack: vec![],
        };

        let json = serde_json::to_string(&snap).unwrap();
        let parsed: StateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, parsed);
    }

    #[test]
    fn divergence_serialization() {
        let div = Divergence {
            snapshot_index: 5,
            turn: 3,
            phase: "Main1".into(),
            field: "players[0].life".into(),
            rust_value: "18".into(),
            java_value: "20".into(),
        };
        let json = serde_json::to_string_pretty(&div).unwrap();
        assert!(json.contains("players[0].life"));
    }
}
