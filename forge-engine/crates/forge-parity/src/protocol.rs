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
    /// Game variant (e.g., "Constructed", "Commander", "Oathbreaker").
    #[serde(default = "default_variant")]
    pub variant: String,
    /// Commander card names (for Commander variants).
    #[serde(default)]
    pub commanders: Vec<String>,
    pub snapshots: Vec<StateSnapshot>,
    #[serde(default)]
    pub decisions: Vec<DecisionRecord>,
    /// Card names that were played/cast during the game.
    pub covered_cards: Vec<String>,
    /// Low-effort mechanic signals extracted from notify messages.
    pub mechanic_signals: Vec<MechanicSignal>,
}

fn default_variant() -> String {
    "Constructed".to_string()
}

/// A normalized decision record emitted at a choice point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecord {
    pub turn: u32,
    pub phase: String,
    pub deciding_player: u32,
    pub kind: String,
    pub options: Vec<String>,
    pub choice: String,
}

/// Low-effort mechanic signal observed during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicSignal {
    pub label: String,
    pub count: usize,
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
    Skipped,
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
    pub skip_reason: Option<String>,
    /// Full Rust-side game trace text, attached for failed matchups.
    pub trace: Option<String>,
    /// Full Java-side game trace text, attached for failed matchups.
    pub java_trace: Option<String>,
    /// Card names covered in this matchup (played/cast at least once).
    pub covered_cards: Vec<String>,
    /// Low-effort mechanic signals observed in this matchup.
    pub mechanic_signals: Vec<MechanicSignal>,
    /// If the game ended naturally, the turn it finished on; otherwise None means stopped at max turns.
    pub finished_turn: Option<u32>,
}

/// Aggregate report for all matchups in matrix mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixReport {
    pub total_matchups: usize,
    pub passed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: usize,
    pub seeds: Vec<u64>,
    pub decks: Vec<String>,
    pub max_turns: u32,
    pub results: Vec<MatchupResult>,
}

// ── Continuous Parity Types ──────────────────────────────────────

/// A single run record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: i64,
    pub batch_id: i64,
    pub deck1: String,
    pub deck2: String,
    pub seed: u64,
    pub status: MatchupStatus,
    pub snapshots_compared: usize,
    pub divergence_count: usize,
    pub first_divergence_field: Option<String>,
    pub first_divergence_rust: Option<String>,
    pub first_divergence_java: Option<String>,
    pub covered_cards: Vec<String>,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub rust_trace: Option<String>,
    pub java_trace: Option<String>,
    pub is_fuzz: bool,
    pub timestamp: String,
    /// Git commit SHA that produced this run result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

/// Aggregate statistics for the continuous parity server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousStats {
    pub total_games: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub pass_rate: f64,
    pub games_per_minute: f64,
    pub uptime_seconds: u64,
    pub current_batch: i64,
    pub fuzz_total: usize,
    pub fuzz_passed: usize,
    pub fuzz_failed: usize,
    pub fuzz_pass_rate: f64,
    /// Git commit SHA the server was built from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

/// A single point in a time-series trend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub bucket: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub pass_rate: f64,
}

/// Deck pair heatmap entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckPairStats {
    pub deck1: String,
    pub deck2: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
    pub pass_rate: f64,
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

        let covered_cards: Vec<String> = vec!["Lightning Bolt".into()];
        let json = serde_json::to_string(&snap).unwrap();
        let parsed: StateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, parsed);
        assert_eq!(covered_cards.len(), 1);
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
