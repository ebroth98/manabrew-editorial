//! ParityRunner: orchestrates game execution and snapshot collection.
//!
//! Loads decks, sets up `GameState` + `GameLoop` with a fixed RNG seed,
//! captures a [`StateSnapshot`] after each phase, and collects them into a
//! [`GameTrace`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use forge_carddb::CardDatabase;
use forge_engine_core::agent::{
    BinaryChoiceKind, GameEntity, ManaCostAction, PlayCardMode, PlayOption, PlayerAgent,
};
use forge_engine_core::card::CardInstance;
use forge_engine_core::combat::DefenderId;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::spellability::AlternativeCost;
use forge_foundation::ZoneType;
use rand::rngs::StdRng;
use rand::SeedableRng;

use serde::Deserialize;

use crate::deck_generator;
use crate::deterministic_agent::DeterministicAgent;
use crate::java_random::JavaRandom;
use crate::parity_card_map::ParityCardMap;
use crate::parity_id;
use crate::parity_order;
use crate::perf;
use crate::protocol::{DecisionRecord, GameTrace, MechanicSignal, StateSnapshot};
use crate::snapshot::snapshot_game;

// ── Preset Deck Loading (from preset_decks/*.json) ───────────────

/// Default directory for preset deck JSON files (relative to CWD).
pub const DEFAULT_DECKS_DIR: &str = "preset_decks";

/// JSON schema for a single card entry in a preset deck file.
#[derive(Debug, Deserialize)]
struct DeckCardEntry {
    name: String,
    count: usize,
}

/// JSON schema for a preset deck file (only the fields parity needs).
#[derive(Debug, Deserialize)]
struct PresetDeckFile {
    cards: Vec<DeckCardEntry>,
}

/// Load a preset deck from `{decks_dir}/{name}.json`, returning (card_name, count) pairs.
fn load_preset_deck(name: &str, decks_dir: &str) -> Result<Vec<(String, usize)>, String> {
    let path = std::path::Path::new(decks_dir).join(format!("{}.json", name));
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read preset deck '{}': {}", path.display(), e))?;
    let deck: PresetDeckFile = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))?;
    Ok(deck.cards.into_iter().map(|c| (c.name, c.count)).collect())
}

/// All available preset deck IDs, derived from JSON files in `decks_dir`.
pub fn available_presets(decks_dir: &str) -> Vec<String> {
    let dir = std::path::Path::new(decks_dir);
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// Resolve a deck spec string to a list of (card_name, count) pairs.
///
/// Supports:
/// - `"inline:Name*Count|Name*Count|..."` — inline deck specification
/// - `"file:/path/to/deck.txt"` — load from a text file (one `Count CardName` per line)
/// - `"red_burn"` etc. — preset deck name lookup from `decks_dir`
pub fn resolve_deck_spec(spec: &str, decks_dir: &str) -> Result<Vec<(String, usize)>, String> {
    if let Some(inline) = spec.strip_prefix("inline:") {
        deck_generator::parse_inline(inline)
    } else if let Some(path) = spec.strip_prefix("file:") {
        parse_deck_file(path)
    } else {
        load_preset_deck(spec, decks_dir)
    }
}

/// Parse a deck list text file. Each line is `Count CardName`, e.g.:
///
/// ```text
/// 4 Lightning Bolt
/// 17 Mountain
/// 1 Zuko, Firebending Master
/// # this is a comment
/// ```
///
/// Blank lines and lines starting with `#` are ignored.
fn parse_deck_file(path: &str) -> Result<Vec<(String, usize)>, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let mut deck = Vec::new();
    for (line_num, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on first whitespace: "4 Lightning Bolt" -> ("4", "Lightning Bolt")
        let (count_str, name) = line.split_once(char::is_whitespace).ok_or_else(|| {
            format!(
                "Line {}: expected 'Count CardName', got '{}'",
                line_num + 1,
                line
            )
        })?;
        let count: usize = count_str.trim().parse().map_err(|_| {
            format!(
                "Line {}: invalid count '{}' in '{}'",
                line_num + 1,
                count_str,
                line
            )
        })?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!(
                "Line {}: empty card name in '{}'",
                line_num + 1,
                line
            ));
        }
        deck.push((name.to_string(), count));
    }
    if deck.is_empty() {
        return Err(format!("Deck file '{}' contains no cards", path));
    }
    Ok(deck)
}

// ── Card Instance Builder ──────────────────────────────────────────

/// Build a deck from a resolved spec. Used by inline/fuzz decks and presets.
fn build_deck_from_spec(
    game: &mut GameState,
    db: &CardDatabase,
    owner: PlayerId,
    spec: &[(String, usize)],
    verbose: bool,
) {
    for (name, count) in spec {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let card = CardInstance::from_rules(rules, owner);
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => {
                if verbose {
                    eprintln!("[parity] Unknown card '{}' — skipped", name);
                }
            }
        }
    }
}

// ── Snapshot-Capturing Agent Wrapper ───────────────────────────────

/// Wraps a `DeterministicAgent` and captures turn-start snapshots.
///
/// The game loop calls `snapshot_state()` then `notify_turn_changed()` at
/// the top of each turn (after `new_turn_for_player()` resets per-turn state).
/// We cache the snapshot in `snapshot_state()` and push it to the shared vec
/// in `notify_turn_changed()` — this matches Java's `GameEventTurnBegan` timing
/// exactly.
struct CapturingAgent {
    player_id: PlayerId,
    inner: DeterministicAgent,
    /// Shared snapshot storage — collected after the game ends.
    shared_snapshots: Arc<Mutex<Vec<StateSnapshot>>>,
    /// Shared coverage storage — card names played/cast at least once.
    shared_covered_cards: Arc<Mutex<BTreeSet<String>>>,
    /// Shared low-effort mechanic signals (notify message buckets).
    shared_mechanic_signals: Arc<Mutex<BTreeMap<String, usize>>>,
    /// Shared choice-point decisions captured from agent callbacks.
    shared_decisions: Arc<Mutex<Vec<DecisionRecord>>>,
    /// Snapshot cached by `snapshot_state()`, pushed on `notify_turn_changed()`.
    pending_snapshot: Option<StateSnapshot>,
    /// If true, capture snapshots at turn start.
    capture_snapshots: bool,
    /// Current turn for decision records.
    current_turn: u32,
    /// Current phase for decision records.
    current_phase: String,
    /// Card id -> card name lookup cache for option labels.
    card_names: HashMap<CardId, String>,
    /// Card id -> is_land cache for main action labels.
    card_is_land: HashMap<CardId, bool>,
    /// (CardId, ability_index) -> is_mana_ability cache for main action labels.
    ability_is_mana: HashMap<(CardId, usize), bool>,
    /// Native Rust CardId -> shared parity id mapping.
    parity_map: Arc<ParityCardMap>,
    /// Latest game snapshot for legality checks in blocker choice logging.
    last_game_state: Option<GameState>,
    /// If true, include verbose-only auxiliary decision records.
    verbose: bool,
    /// Active mana-payment callback session, if we are in a multi-step payment loop.
    active_mana_payment: Option<(u32, String, String)>,
}

impl CapturingAgent {
    fn new(
        player_id: PlayerId,
        verbose: bool,
        prefer_actions: bool,
        shared: Arc<Mutex<Vec<StateSnapshot>>>,
        covered: Arc<Mutex<BTreeSet<String>>>,
        mechanics: Arc<Mutex<BTreeMap<String, usize>>>,
        decisions: Arc<Mutex<Vec<DecisionRecord>>>,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        parity_map: Arc<ParityCardMap>,
        capture_snapshots: bool,
    ) -> Self {
        Self {
            player_id,
            inner: DeterministicAgent::new(
                player_id,
                verbose,
                rng,
                game_rng,
                prefer_actions,
                Arc::clone(&parity_map),
            ),
            shared_snapshots: shared,
            shared_covered_cards: covered,
            shared_mechanic_signals: mechanics,
            shared_decisions: decisions,
            pending_snapshot: None,
            capture_snapshots,
            current_turn: 0,
            current_phase: "Unknown".to_string(),
            card_names: HashMap::new(),
            card_is_land: HashMap::new(),
            ability_is_mana: HashMap::new(),
            parity_map,
            last_game_state: None,
            verbose,
            active_mana_payment: None,
        }
    }

    fn card_name(&self, id: CardId) -> String {
        self.card_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Card({})", id.0))
    }

    fn card_label(&self, id: CardId) -> String {
        format!("{}@{}", self.card_name(id), self.parity_map.id(id))
    }

    fn record_decision(&self, kind: &str, options: Vec<String>, choice: String) {
        if self.verbose {
            let rng_count = self.inner.rng_call_count();
            eprintln!(
                "[decision rng#{} P{} {} {}]",
                rng_count, self.player_id.0, kind, choice
            );
        }
        self.shared_decisions.lock().unwrap().push(DecisionRecord {
            turn: self.current_turn,
            phase: self.current_phase.clone(),
            deciding_player: self.player_id.0,
            kind: kind.to_string(),
            options,
            choice,
        });
    }

    fn record_verbose_decision(&self, kind: &str, options: Vec<String>, choice: String) {
        if self.verbose {
            self.record_decision(kind, options, choice);
        }
    }

    fn legal_attackers_for_blocker(&self, blocker: CardId, attackers: &[CardId]) -> Vec<CardId> {
        let Some(ref game) = self.last_game_state else {
            return attackers.to_vec();
        };
        attackers
            .iter()
            .copied()
            .filter(|&attacker| {
                forge_engine_core::combat::can_creature_block(game, blocker, attacker)
            })
            .collect()
    }
}

impl PlayerAgent for CapturingAgent {
    fn snapshot_state(
        &mut self,
        game: &GameState,
        mana_pools: &[forge_engine_core::mana::ManaPool],
    ) {
        let t_total = Instant::now();
        self.inner.snapshot_state(game, mana_pools);
        self.card_names.clear();
        self.card_is_land.clear();
        self.ability_is_mana.clear();
        let t_clone = Instant::now();
        self.last_game_state = Some(game.clone());
        perf::record("agent.snapshot_state.clone_game", t_clone.elapsed());

        let t_cache = Instant::now();
        for c in &game.cards {
            let name = if c.face_down {
                String::new()
            } else {
                c.card_name.clone()
            };
            self.card_names.insert(c.id, name);
            self.card_is_land.insert(c.id, c.is_land());
            for ab in &c.activated_abilities {
                self.ability_is_mana
                    .insert((c.id, ab.ability_index), ab.is_mana_ability);
            }
        }
        perf::record(
            "agent.snapshot_state.rebuild_card_caches",
            t_cache.elapsed(),
        );
        if !self.capture_snapshots {
            perf::record("agent.snapshot_state.total", t_total.elapsed());
            return;
        }
        // Cache the snapshot — it will be pushed when notify_turn_changed fires
        let t_snap = Instant::now();
        self.pending_snapshot = Some(snapshot_game(game));
        perf::record("agent.snapshot_state.snapshot_game", t_snap.elapsed());
        perf::record("agent.snapshot_state.total", t_total.elapsed());
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        self.inner.notify_turn_changed(active_player, turn_number);
        self.current_turn = turn_number;
        if !self.capture_snapshots {
            return;
        }
        // Push the pending snapshot captured by the preceding snapshot_state() call.
        // This fires after new_turn_for_player() has reset per-turn state but before
        // any actions — matching Java's GameEventTurnBegan.
        //
        // Override the phase to "Untap" because new_turn_for_player doesn't update
        // the phase (it still shows the previous turn's Cleanup). Java's snapshot
        // at GameEventTurnBegan has phase=Untap because setPhase(UNTAP) is called
        // before the event fires.
        if let Some(mut snap) = self.pending_snapshot.take() {
            snap.phase = "Untap".to_string();
            // Normalize lands_played for non-active players to 0.
            // Java's incrementTurn() resets ALL players' landsPlayedThisTurn,
            // but Rust's new_turn_for_player() only resets the active player's.
            // Since non-active players can't play lands, the value is irrelevant
            // for gameplay — we normalize to match Java's behavior.
            let active = snap.active_player as usize;
            for (i, p) in snap.players.iter_mut().enumerate() {
                if i != active {
                    p.lands_played = 0;
                }
            }
            self.shared_snapshots.lock().unwrap().push(snap);
        }
    }

    fn notify_phase_changed(&mut self, phase: forge_foundation::PhaseType) {
        self.current_phase = format!("{:?}", phase);
        if self.verbose {
            let rng_count = self.inner.rng_call_count();
            eprintln!(
                "[phase P{} {:?} rng#{}]",
                self.player_id.0, phase, rng_count
            );
        }
        self.inner.notify_phase_changed(phase);
    }

    fn on_library_peek(&mut self, game: &GameState, cards: &[CardId]) {
        self.inner.on_library_peek(game, cards);
    }

    fn mulligan_decision(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        mulligan_count: u32,
    ) -> bool {
        self.inner.mulligan_decision(player, hand, mulligan_count)
    }

    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[PlayOption],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> forge_engine_core::player::actions::PlayerAction {
        #[derive(Clone, Copy)]
        enum EntryKind {
            Card(PlayOption),
            Ability(CardId, usize),
        }
        let mut entries: Vec<(String, EntryKind)> = Vec::new();
        for &play in playable {
            let cid = play.card_id;
            let base = if self.card_is_land.get(&cid).copied().unwrap_or(false) {
                format!("LAND:{}", self.card_name(cid))
            } else {
                let fb_tag = match play.mode {
                    PlayCardMode::Alternative(AlternativeCost::Flashback) => "[FB]",
                    _ => "",
                };
                format!("SPELL:{}{}", self.card_name(cid), fb_tag)
            };
            entries.push((base, EntryKind::Card(play)));
        }
        for &(cid, ab_idx) in activatable {
            if self
                .ability_is_mana
                .get(&(cid, ab_idx))
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            entries.push((
                format!("AB:{}", self.card_name(cid)),
                EntryKind::Ability(cid, ab_idx),
            ));
        }
        // Sort entries using a concatenated pipe-delimited string key, matching
        // Java's ParityOrder.actionSortKey() which concatenates fields into a single
        // string.  Using tuple comparison would differ for multi-digit parity IDs
        // because tuple compares element-by-element ("1" < "11") while concatenated
        // string comparison sees "1|" > "11|" (since '|' > '1').
        entries.sort_by(|a, b| {
            let key = |(label, kind): &(String, EntryKind)| -> String {
                match *kind {
                    EntryKind::Card(cid) => {
                        let variant = match cid.mode {
                            PlayCardMode::Normal => "0".to_string(),
                            PlayCardMode::Alternative(AlternativeCost::Flashback) => {
                                "Flashback".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Spectacle) => {
                                "Spectacle".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Evoke) => {
                                "Evoke".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Dash) => "Dash".to_string(),
                            PlayCardMode::Alternative(AlternativeCost::Blitz) => {
                                "Blitz".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Escape) => {
                                "Escape".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Overload) => {
                                "Overload".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Madness) => {
                                "Madness".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Foretell) => {
                                "Foretell".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Emerge) => {
                                "Emerge".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Suspend) => {
                                "Suspend".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Morph)
                            | PlayCardMode::Alternative(AlternativeCost::Megamorph) => {
                                "Morph".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Bestow) => {
                                "Bestow".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Warp) => "0".to_string(),
                            PlayCardMode::Alternative(AlternativeCost::SacrificeAlt) => {
                                "0".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Plot) => "Plot".to_string(),
                            PlayCardMode::Alternative(AlternativeCost::Awaken) => {
                                "Awaken".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Disturb) => {
                                "Disturb".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Harmonize) => {
                                "Harmonize".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Freerunning) => {
                                "Freerunning".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Impending) => {
                                "Impending".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Mayhem) => {
                                "Mayhem".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::MTMtE) => {
                                "MTMtE".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Mutate) => {
                                "Mutate".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Prowl) => {
                                "Prowl".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Sneak) => {
                                "Sneak".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Surge) => {
                                "Surge".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::WebSlinging) => {
                                "WebSlinging".to_string()
                            }
                            PlayCardMode::Alternative(AlternativeCost::Plotted) => {
                                "Plotted".to_string()
                            }
                            // Host-card `Mode$ AlternativeCost` actions are
                            // represented in Rust as `StaticAlternative`.
                            PlayCardMode::StaticAlternative => "StaticAlternative".to_string(),
                            PlayCardMode::ForetellExile => "ForetellExile".to_string(),
                        };
                        let fallback = match cid.mode {
                            PlayCardMode::Normal => "Normal",
                            PlayCardMode::Alternative(AlternativeCost::Warp) => "Warp",
                            PlayCardMode::StaticAlternative => "StaticAlternative",
                            _ => "",
                        };
                        format!(
                            "{}|0|{}|{}|{}",
                            label,
                            self.parity_map.id(cid.card_id),
                            variant,
                            fallback
                        )
                    }
                    EntryKind::Ability(cid, idx) => {
                        format!("{}|1|{}|{:05}", label, self.parity_map.id(cid), idx)
                    }
                }
            };
            key(a).cmp(&key(b))
        });
        let mut options_raw: Vec<String> = entries.iter().map(|(s, _)| s.clone()).collect();
        // Mirror Java harness: if the same card has multiple cost variants, label as $1/$2/...
        let mut totals: HashMap<(String, u64), usize> = HashMap::new();
        for (idx, (_, kind)) in entries.iter().enumerate() {
            let key = match *kind {
                EntryKind::Card(play) => self.parity_map.id(play.card_id) as u64,
                EntryKind::Ability(cid, _) => self.parity_map.id(cid) as u64,
            };
            *totals.entry((options_raw[idx].clone(), key)).or_insert(0) += 1;
        }
        let mut seen: HashMap<(String, u64), usize> = HashMap::new();
        for i in 0..entries.len() {
            let key = match entries[i].1 {
                EntryKind::Card(play) => self.parity_map.id(play.card_id) as u64,
                EntryKind::Ability(cid, _) => self.parity_map.id(cid) as u64,
            };
            let tuple = (options_raw[i].clone(), key);
            if totals.get(&tuple).copied().unwrap_or(0) > 1 {
                let n = seen.get(&tuple).copied().unwrap_or(0) + 1;
                seen.insert(tuple.clone(), n);
                options_raw[i] = format!("{}${}", options_raw[i], n);
            }
        }
        let option_keys: Vec<u64> = entries
            .iter()
            .map(|(_, kind)| match *kind {
                EntryKind::Card(play) => self.parity_map.id(play.card_id) as u64,
                EntryKind::Ability(cid, _idx) => self.parity_map.id(cid) as u64,
            })
            .collect();
        let options = parity_id::disambiguate_labels_with_keys(&options_raw, &option_keys);

        let action = self.inner.choose_action(
            player,
            playable,
            tappable_lands,
            untappable_lands,
            activatable,
        );

        if entries.is_empty() {
            return action;
        }

        let choice = match action {
            forge_engine_core::player::actions::PlayerAction::PassPriority => "PASS".to_string(),
            forge_engine_core::player::actions::PlayerAction::CastSpell(play) => entries
                .iter()
                .enumerate()
                .find(|(_, (_, kind))| matches!(kind, EntryKind::Card(id) if *id == play))
                .map(|(idx, _)| options[idx].clone())
                .unwrap_or_else(|| {
                    let cid = play.card_id;
                    if self.card_is_land.get(&cid).copied().unwrap_or(false) {
                        format!("LAND:{}", self.card_name(cid))
                    } else {
                        let fb_tag = match play.mode {
                            PlayCardMode::Alternative(AlternativeCost::Flashback) => "[FB]",
                            _ => "",
                        };
                        format!("SPELL:{}{}", self.card_name(cid), fb_tag)
                    }
                }),
            forge_engine_core::player::actions::PlayerAction::ActivateMana(cid) => {
                format!("MANA:{}", self.card_name(cid))
            }
            forge_engine_core::player::actions::PlayerAction::UndoMana(cid) => {
                format!("UNTAP_MANA:{}", self.card_name(cid))
            }
            forge_engine_core::player::actions::PlayerAction::ActivateAbility(ability) => entries
                .iter()
                .enumerate()
                .find(|(_, (_, kind))| {
                    matches!(
                        kind,
                        EntryKind::Ability(id, idx)
                            if *id == ability.card_id && *idx == ability.ability_index
                    )
                })
                .map(|(idx, _)| options[idx].clone())
                .unwrap_or_else(|| {
                    format!(
                        "AB:{}@{}",
                        self.card_name(ability.card_id),
                        self.parity_map.id(ability.card_id)
                    )
                }),
            _ => "PASS".to_string(),
        };
        self.record_decision("main_action", options, choice);
        action
    }

    fn choose_attackers(
        &mut self,
        player: PlayerId,
        available: &[CardId],
        possible_defenders: &[DefenderId],
    ) -> Vec<(CardId, DefenderId)> {
        let mut sorted_defenders: Vec<DefenderId> = possible_defenders.to_vec();
        sorted_defenders.sort_by_key(|d| format!("{:?}", d));
        let sorted_available = parity_order::sort_cards_by_name_then_id(
            available,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let picked = self
            .inner
            .choose_attackers(player, &sorted_available, &sorted_defenders);
        let attacker_labels = parity_id::label_cards_in_order(
            &sorted_available,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let mut attacker_label_by_id: HashMap<CardId, String> = HashMap::new();
        for (id, label) in attacker_labels {
            attacker_label_by_id.insert(id, label);
        }
        for &attacker in &sorted_available {
            let mut options = vec!["PASS".to_string()];
            let attacker_label = attacker_label_by_id
                .get(&attacker)
                .cloned()
                .unwrap_or_else(|| self.card_name(attacker));
            for (idx, _) in sorted_defenders.iter().enumerate() {
                options.push(format!("ATTACK:{attacker_label}->D{idx}"));
            }

            let choice = picked
                .iter()
                .find(|(cid, _)| *cid == attacker)
                .and_then(|(_, defender)| {
                    sorted_defenders
                        .iter()
                        .position(|d| d == defender)
                        .map(|idx| format!("ATTACK:{attacker_label}->D{idx}"))
                })
                .unwrap_or_else(|| "PASS".to_string());

            self.record_decision("combat_attacker_choice", options, choice);
        }

        picked
    }

    fn exert_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        self.inner.exert_attackers(player, attackers)
    }

    fn enlist_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        self.inner.enlist_attackers(player, attackers)
    }

    fn choose_blockers(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
        max_blockers: Option<usize>,
    ) -> Vec<(CardId, CardId)> {
        let sorted_attackers = parity_order::sort_cards_by_name_then_id(
            attackers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let sorted_blockers = parity_order::sort_cards_by_name_then_id(
            available_blockers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let chosen =
            self.inner
                .choose_blockers(player, &sorted_attackers, &sorted_blockers, max_blockers);

        let attacker_labels = parity_id::label_cards_in_order(
            &sorted_attackers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let blocker_labels = parity_id::label_cards_in_order(
            &sorted_blockers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let attacker_label_by_id: HashMap<CardId, String> = attacker_labels.into_iter().collect();
        let blocker_label_by_id: HashMap<CardId, String> = blocker_labels.into_iter().collect();

        let mut chosen_by_blocker: HashMap<CardId, CardId> = HashMap::new();
        for (blocker, attacker) in chosen.iter().copied() {
            chosen_by_blocker.entry(blocker).or_insert(attacker);
        }

        // Track how many blockers have been declared for BlockRestrict enforcement.
        let mut blockers_declared = 0usize;
        for &blocker in &sorted_blockers {
            let blocker_label = blocker_label_by_id
                .get(&blocker)
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "{}@{}",
                        self.card_name(blocker),
                        self.parity_map.id(blocker)
                    )
                });
            let mut options = vec!["PASS".to_string()];
            let at_limit = max_blockers.map_or(false, |max| blockers_declared >= max);
            let legal_attackers = if at_limit {
                Vec::new() // BlockRestrict reached → forced PASS, no options
            } else {
                self.legal_attackers_for_blocker(blocker, &sorted_attackers)
            };
            for &attacker in &legal_attackers {
                let attacker_label = attacker_label_by_id
                    .get(&attacker)
                    .cloned()
                    .unwrap_or_else(|| self.card_name(attacker));
                options.push(format!("BLOCK:{blocker_label}->{attacker_label}"));
            }
            let choice = chosen_by_blocker
                .get(&blocker)
                .and_then(|attacker| {
                    attacker_label_by_id
                        .get(attacker)
                        .map(|label| format!("BLOCK:{blocker_label}->{label}"))
                })
                .unwrap_or_else(|| "PASS".to_string());
            if chosen_by_blocker.contains_key(&blocker) {
                blockers_declared += 1;
            }
            self.record_decision("combat_blocker_choice", options, choice);
        }

        chosen
    }

    fn choose_blocker_for(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        blocker: CardId,
    ) -> Option<CardId> {
        let sorted_attackers = parity_order::sort_cards_by_name_then_id(
            attackers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let chosen = self
            .inner
            .choose_blocker_for(player, &sorted_attackers, blocker);
        let attacker_labels = parity_id::label_cards_in_order(
            &sorted_attackers,
            |id| self.card_name(id),
            |id| self.parity_map.id(id),
        );
        let mut attacker_label_by_id: HashMap<CardId, String> = HashMap::new();
        for (id, label) in attacker_labels {
            attacker_label_by_id.insert(id, label);
        }
        let mut options = vec!["PASS".to_string()];
        let blocker_label = format!(
            "{}@{}",
            self.card_name(blocker),
            self.parity_map.id(blocker)
        );
        let legal_attackers = self.legal_attackers_for_blocker(blocker, &sorted_attackers);
        for &attacker in &legal_attackers {
            let attacker_label = attacker_label_by_id
                .get(&attacker)
                .cloned()
                .unwrap_or_else(|| self.card_name(attacker));
            options.push(format!("BLOCK:{blocker_label}->{attacker_label}",));
        }
        let choice = chosen
            .map(|attacker| {
                let attacker_label = attacker_label_by_id
                    .get(&attacker)
                    .cloned()
                    .unwrap_or_else(|| self.card_name(attacker));
                format!("BLOCK:{blocker_label}->{attacker_label}")
            })
            .unwrap_or_else(|| "PASS".to_string());
        self.record_decision("combat_blocker_choice", options, choice);

        chosen
    }

    fn choose_damage_assignment_order(
        &mut self,
        player: PlayerId,
        attacker: CardId,
        blockers: &[CardId],
    ) -> Vec<CardId> {
        self.inner
            .choose_damage_assignment_order(player, attacker, blockers)
    }

    fn assign_combat_damage(
        &mut self,
        game: &GameState,
        player: PlayerId,
        attacker: CardId,
        blockers_in_order: &[CardId],
        defender: Option<forge_engine_core::combat::DefenderId>,
        damage_to_assign: i32,
    ) -> Vec<(Option<CardId>, i32)> {
        self.inner.assign_combat_damage(
            game,
            player,
            attacker,
            blockers_in_order,
            defender,
            damage_to_assign,
        )
    }

    fn choose_target_player(
        &mut self,
        player: PlayerId,
        valid: &[PlayerId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<PlayerId> {
        self.inner.choose_target_player(player, valid, sa)
    }

    fn choose_target_card(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        self.inner.choose_target_card(player, valid, sa)
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        zone: ZoneType,
        valid: &[CardId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        self.inner
            .choose_target_card_from_zone(player, zone, valid, sa)
    }

    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> forge_engine_core::agent::TargetChoice {
        self.inner
            .choose_target_any(player, valid_players, valid_cards, sa)
    }

    fn choose_legend_keep(&mut self, player: PlayerId, duplicates: &[CardId]) -> CardId {
        self.inner.choose_legend_keep(player, duplicates)
    }

    fn choose_sacrifice(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        self.inner.choose_sacrifice(player, valid, sa)
    }

    fn choose_type(
        &mut self,
        player: PlayerId,
        type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
        self.inner.choose_type(player, type_category, valid_types)
    }

    fn choose_scry(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        self.inner.choose_scry(player, cards)
    }

    fn choose_surveil(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        self.inner.choose_surveil(player, cards)
    }

    fn choose_dig(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        max: usize,
        optional: bool,
    ) -> Vec<CardId> {
        let chosen = self.inner.choose_dig(player, valid, max, optional);
        let options: Vec<String> = valid
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        let picked: Vec<String> = chosen
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        self.record_decision(
            "choose_dig",
            options,
            if picked.is_empty() {
                "PASS".to_string()
            } else {
                picked.join(",")
            },
        );
        chosen
    }

    fn choose_reorder_library(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        self.inner.choose_reorder_library(player, cards)
    }

    fn choose_discard(&mut self, player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        self.inner.choose_discard(player, hand, num)
    }

    fn choose_random_discard(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        self.inner.choose_random_discard(player, hand, num)
    }

    fn choose_cards_for_effect(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        let chosen = self.inner.choose_cards_for_effect(player, valid, min, max);
        let options: Vec<String> = valid
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        let picked: Vec<String> = chosen
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        self.record_verbose_decision(
            "choose_cards_for_effect",
            options,
            if picked.is_empty() {
                "PASS".to_string()
            } else {
                picked.join(",")
            },
        );
        chosen
    }

    fn choose_entities_for_effect(
        &mut self,
        player: PlayerId,
        candidates: &[GameEntity],
        min: usize,
        max: usize,
    ) -> Vec<GameEntity> {
        let chosen = self
            .inner
            .choose_entities_for_effect(player, candidates, min, max);
        let format_entity = |e: &GameEntity| -> String {
            match e {
                GameEntity::Player(pid) => format!("Player({})", pid.index()),
                GameEntity::Card(cid) => {
                    let name = self.card_name(*cid);
                    format!("{name}@{}", self.parity_map.id(*cid))
                }
            }
        };
        let options: Vec<String> = candidates.iter().map(format_entity).collect();
        let picked: Vec<String> = chosen.iter().map(format_entity).collect();
        self.record_verbose_decision(
            "choose_entities_for_effect",
            options,
            if picked.is_empty() {
                "PASS".to_string()
            } else {
                picked.join(",")
            },
        );
        chosen
    }

    fn choose_single_card_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        select_prompt: &str,
        is_optional: bool,
    ) -> Option<CardId> {
        let chosen = self.inner.choose_single_card_for_zone_change(
            player,
            valid,
            select_prompt,
            is_optional,
        );
        let options: Vec<String> = valid
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        let picked = chosen
            .map(|cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .unwrap_or_else(|| "PASS".to_string());
        self.record_verbose_decision("choose_zone_change", options, picked);
        chosen
    }

    fn choose_cards_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
        _select_prompt: &str,
    ) -> Vec<CardId> {
        let chosen =
            self.inner
                .choose_cards_for_zone_change(player, valid, min, max, _select_prompt);
        let options: Vec<String> = valid
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        let picked: Vec<String> = chosen
            .iter()
            .map(|&cid| {
                let name = self.card_name(cid);
                format!("{name}@{}", self.parity_map.id(cid))
            })
            .collect();
        self.record_verbose_decision(
            "choose_zone_change",
            options,
            if picked.is_empty() {
                "PASS".to_string()
            } else {
                picked.join(",")
            },
        );
        chosen
    }

    fn choose_target_spell(&mut self, player: PlayerId, valid: &[u32]) -> Option<u32> {
        self.inner.choose_target_spell(player, valid)
    }

    fn choose_mode(
        &mut self,
        player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<usize> {
        self.inner
            .choose_mode(player, descriptions, min, max, card_name)
    }

    fn choose_x_value(&mut self, player: PlayerId, max_x: u32, card_name: Option<&str>) -> u32 {
        self.inner.choose_x_value(player, max_x, card_name)
    }

    fn pay_x_cost_in_mana(&self) -> bool {
        self.inner.pay_x_cost_in_mana()
    }

    fn choose_optional_trigger(
        &mut self,
        player: PlayerId,
        description: &str,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = self
            .inner
            .choose_optional_trigger(player, description, card_name, api);
        if std::env::var("FORGE_TRIGGER_TRACE").is_ok() {
            eprintln!("[trigger-trace] choose_optional_trigger: player={} desc={:?} card={:?} api={:?} -> {}", player.0, description, card_name, api, accept);
        }
        self.record_decision(
            "optional_trigger",
            vec!["DECLINE".to_string(), "ACCEPT".to_string()],
            if accept {
                "ACCEPT".to_string()
            } else {
                "DECLINE".to_string()
            },
        );
        accept
    }

    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool> {
        self.inner.choose_land_or_spell(player)
    }

    fn confirm_action(
        &mut self,
        player: PlayerId,
        mode: Option<&str>,
        message: &str,
        options: &[String],
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = self
            .inner
            .confirm_action(player, mode, message, options, card_name, api);
        let mut logged_options = if options.is_empty() {
            vec!["DECLINE".to_string(), "ACCEPT".to_string()]
        } else {
            options.to_vec()
        };
        if logged_options.is_empty() {
            logged_options = vec!["DECLINE".to_string(), "ACCEPT".to_string()];
        }
        self.record_decision(
            "confirm_action",
            logged_options,
            if accept {
                "ACCEPT".to_string()
            } else {
                "DECLINE".to_string()
            },
        );
        accept
    }

    fn confirm_payment(
        &mut self,
        player: PlayerId,
        cost_kind: &str,
        message: &str,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = self
            .inner
            .confirm_payment(player, cost_kind, message, card_name, api);
        self.record_decision(
            "confirm_payment",
            vec!["DECLINE".to_string(), "ACCEPT".to_string()],
            if accept {
                "ACCEPT".to_string()
            } else {
                "DECLINE".to_string()
            },
        );
        accept
    }

    fn confirm_replacement_effect(
        &mut self,
        player: PlayerId,
        question: &str,
        effect_description: &str,
        card_name: Option<&str>,
    ) -> bool {
        let accept =
            self.inner
                .confirm_replacement_effect(player, question, effect_description, card_name);
        self.record_decision(
            "confirm_replacement_effect",
            vec!["DECLINE".to_string(), "ACCEPT".to_string()],
            if accept {
                "ACCEPT".to_string()
            } else {
                "DECLINE".to_string()
            },
        );
        accept
    }

    fn choose_binary(
        &mut self,
        player: PlayerId,
        question: &str,
        kind: BinaryChoiceKind,
        default_choice: Option<bool>,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let left = format!("{}:LEFT", kind.as_str());
        let right = format!("{}:RIGHT", kind.as_str());
        let chosen_left =
            self.inner
                .choose_binary(player, question, kind, default_choice, card_name, api);
        self.record_decision(
            "choose_binary",
            vec![left.clone(), right.clone()],
            if chosen_left { left } else { right },
        );
        chosen_left
    }

    fn choose_color(&mut self, player: PlayerId, valid_colors: &[String]) -> Option<String> {
        let chosen = self.inner.choose_color(player, valid_colors);
        self.record_verbose_decision(
            "choose_color",
            valid_colors.to_vec(),
            chosen.clone().unwrap_or_else(|| "NONE".to_string()),
        );
        chosen
    }

    fn choose_card_name(&mut self, player: PlayerId, valid_names: &[String]) -> Option<String> {
        let chosen = self.inner.choose_card_name(player, valid_names);
        self.record_verbose_decision(
            "choose_card_name",
            valid_names.to_vec(),
            chosen.clone().unwrap_or_else(|| "NONE".to_string()),
        );
        chosen
    }

    fn choose_number(&mut self, player: PlayerId, min: i32, max: i32) -> Option<i32> {
        let chosen = self.inner.choose_number(player, min, max);
        self.record_verbose_decision(
            "choose_number",
            vec![format!("{}..{}", min, max)],
            chosen.map_or("NONE".to_string(), |v| v.to_string()),
        );
        chosen
    }

    fn notify(&mut self, message: &str) {
        if let Some(card_name) = extract_coverage_card(message) {
            self.shared_covered_cards
                .lock()
                .unwrap()
                .insert(card_name.to_string());
        } else if let Some(label) = extract_mechanic_signal(message) {
            let mut map = self.shared_mechanic_signals.lock().unwrap();
            *map.entry(label).or_insert(0) += 1;
        }
        self.inner.notify(message);
    }

    fn notify_card_played(
        &mut self,
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        set_code: &str,
    ) {
        self.inner
            .notify_card_played(player, card_id, card_name, set_code);
    }

    fn notify_state_changed(&mut self) {
        self.inner.notify_state_changed();
    }

    fn pay_mana_cost(
        &mut self,
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        mana_cost: &str,
        mana_cost_display: &str,
        mana_cost_checkpoint: &str,
        allow_reserved_source_reuse: bool,
        mana_ability_options: &[forge_engine_core::agent::ManaAbilityOption],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        mana_pool: &forge_engine_core::mana::ManaPool,
    ) -> ManaCostAction {
        let session_key = (
            self.current_turn,
            self.current_phase.clone(),
            format!("{}|{}", self.card_label(card_id), mana_cost_checkpoint),
        );
        if self.active_mana_payment.as_ref() != Some(&session_key) {
            self.record_decision(
                "pay_mana_cost_callback",
                vec![self.card_label(card_id), mana_cost_checkpoint.to_string()],
                "CALLBACK".to_string(),
            );
            self.active_mana_payment = Some(session_key.clone());
        }
        let action = self.inner.pay_mana_cost(
            player,
            card_id,
            card_name,
            mana_cost,
            mana_cost_display,
            mana_cost_checkpoint,
            allow_reserved_source_reuse,
            mana_ability_options,
            tappable_lands,
            untappable_lands,
            mana_pool,
        );
        if matches!(action, ManaCostAction::Pay | ManaCostAction::Cancel) {
            self.active_mana_payment = None;
        }
        action
    }

    fn choose_single_replacement_effect(
        &mut self,
        player: PlayerId,
        descriptions: &[String],
    ) -> usize {
        let chosen = self
            .inner
            .choose_single_replacement_effect(player, descriptions);
        self.record_verbose_decision(
            "choose_single_replacement_effect",
            descriptions.to_vec(),
            descriptions
                .get(chosen)
                .cloned()
                .unwrap_or_else(|| format!("Effect#{}", chosen)),
        );
        chosen
    }
}

// ── ParityRunner ───────────────────────────────────────────────────

/// Configuration for a parity run.
pub struct RunConfig {
    pub deck1: String,
    pub deck2: String,
    pub seed: u64,
    pub max_turns: u32,
    pub cards_dir: Option<String>,
    pub decks_dir: Option<String>,
    pub verbose: bool,
    pub prefer_actions: bool,
    /// Maximum JVM heap size (e.g. "512m", "1g").
    pub java_heap: String,
    /// Game variant: "Constructed", "Commander", "Oathbreaker", "TinyLeaders", "Brawl".
    pub variant: String,
    /// Commander card names for Commander variants.
    pub commanders: Vec<String>,
}

/// Pre-loaded card database and token templates, reusable across multiple matchups.
pub struct LoadedData {
    pub db: CardDatabase,
    pub token_templates: Vec<(String, CardInstance)>,
}

/// Load the card database and token templates once.
pub fn load_data(cards_dir: Option<&str>, verbose: bool) -> Result<LoadedData, String> {
    let t_total = Instant::now();
    let cards_dir = cards_dir.unwrap_or("forge/forge-gui/res/cardsfolder");
    let cards_path = std::path::Path::new(cards_dir);

    if !cards_path.exists() {
        return Err(format!(
            "Cards directory not found: {}. Set --cards-dir to the Forge cardsfolder path.",
            cards_dir,
        ));
    }

    if verbose {
        eprintln!("[parity] Loading cards from {:?} ...", cards_path);
    }
    let t_cards = Instant::now();
    let (db, result) = CardDatabase::load_from_directory(cards_path);
    perf::record("load_data.cards_db", t_cards.elapsed());
    if verbose {
        eprintln!(
            "[parity] Loaded {} cards ({} failed)",
            result.loaded, result.failed
        );
    }

    let mut token_templates = Vec::new();
    let token_dir_path = cards_path
        .parent()
        .map(|p| p.join("tokenscripts"))
        .unwrap_or_default();
    if token_dir_path.exists() {
        if verbose {
            eprintln!(
                "[parity] Loading token scripts from {:?} ...",
                token_dir_path
            );
        }
        let t_tokens = Instant::now();
        let (token_db, token_result) = CardDatabase::load_from_directory(&token_dir_path);
        perf::record("load_data.token_db", t_tokens.elapsed());
        if verbose {
            eprintln!("[parity] Loaded {} token scripts", token_result.loaded);
        }
        for (script_name, rules) in token_db.iter() {
            let template = CardInstance::from_rules(rules, PlayerId(0));
            token_templates.push((script_name.clone(), template));
        }
    }

    // Load creature types from TypeLists.txt into the engine's global registry.
    // Mirrors Java's FModel.loadDynamicGamedata() → CardType.Constant.CREATURE_TYPES.
    {
        let type_list_path = cards_path
            .parent()
            .map(|p| p.join("lists").join("TypeLists.txt"))
            .unwrap_or_default();
        if !type_list_path.exists() {
            return Err(format!(
                "TypeLists.txt not found at {:?}. This file is required for creature type data.",
                type_list_path
            ));
        }
        if verbose {
            eprintln!("[parity] Loading type lists from {:?} ...", type_list_path);
        }
        let t_types_read = Instant::now();
        let content = std::fs::read_to_string(&type_list_path).map_err(|e| {
            format!(
                "Failed to read TypeLists.txt at {:?}: {}",
                type_list_path, e
            )
        })?;
        perf::record("load_data.typelist_read", t_types_read.elapsed());
        let t_types_parse = Instant::now();
        forge_engine_core::game::TypeRegistry::load(&content);
        perf::record("load_data.typelist_parse", t_types_parse.elapsed());
        if verbose {
            eprintln!(
                "[parity] Loaded {} creature types",
                forge_engine_core::game::TypeRegistry::creature_types().len()
            );
        }
    }

    perf::record("load_data.total", t_total.elapsed());
    Ok(LoadedData {
        db,
        token_templates,
    })
}

/// Run a game using pre-loaded data (avoids reloading the DB for each matchup).
pub fn run_with_data(config: &RunConfig, data: &LoadedData) -> Result<GameTrace, String> {
    let t_total = Instant::now();
    // Resolve deck lists — supports preset names, inline: specs, and file: specs
    let decks_dir = config.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let t_resolve = Instant::now();
    let deck1_spec = resolve_deck_spec(&config.deck1, decks_dir)?;
    let deck2_spec = resolve_deck_spec(&config.deck2, decks_dir)?;
    perf::record("run_with_data.resolve_deck_spec", t_resolve.elapsed());

    // Determine starting life based on variant
    let starting_life = match config.variant.as_str() {
        "Commander" => 40,
        "Oathbreaker" => 20,
        "TinyLeaders" => 25,
        "Brawl" => 30,
        _ => 20, // Constructed and other variants
    };

    // Reset global state for cross-game isolation (matches Java's resetIdCounter)
    forge_engine_core::spellability::SpellAbility::reset_id_counter();

    // Set up game
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);
    let mut game = GameState::new(&["Player1", "Player2"], starting_life);

    let t_build = Instant::now();
    build_deck_from_spec(&mut game, &data.db, p0, &deck1_spec, config.verbose);
    build_deck_from_spec(&mut game, &data.db, p1, &deck2_spec, config.verbose);
    perf::record("run_with_data.build_deck", t_build.elapsed());

    // Set up commanders if in a commander variant
    let is_commander_variant = matches!(
        config.variant.as_str(),
        "Commander" | "Oathbreaker" | "TinyLeaders" | "Brawl"
    );
    if is_commander_variant && !config.commanders.is_empty() {
        let mut unique_commanders: Vec<&str> = Vec::new();
        let mut seen = HashSet::new();
        for commander_name in &config.commanders {
            let key = commander_name.to_ascii_lowercase();
            if seen.insert(key) {
                unique_commanders.push(commander_name.as_str());
            }
        }

        // For each player, find commander cards already present in library and move to command zone.
        // Contract: --commander names must already be in the main deck list.
        for &pid in &[p0, p1] {
            for commander_name in &unique_commanders {
                let card_id = game
                    .cards_in_zone(ZoneType::Library, pid)
                    .iter()
                    .copied()
                    .find(|&cid| {
                        game.card(cid)
                            .card_name
                            .eq_ignore_ascii_case(commander_name)
                    })
                    .ok_or_else(|| {
                        format!(
                            "Commander \"{}\" not found in player {} main deck/library",
                            commander_name,
                            pid.0 + 1
                        )
                    })?;

                // Move to command zone and register as commander.
                game.move_card(card_id, ZoneType::Command, pid);
                game.player_register_commander(pid, card_id);
            }
        }
        // Set commander_damage_enabled based on variant
        let commander_damage_enabled = config.variant == "Commander";
        for &pid in &[p0, p1] {
            game.player_mut(pid).commander_damage_enabled = commander_damage_enabled;
        }
    }

    let mut game_loop = GameLoop::new(2);

    // Register token templates
    for (script_name, template) in &data.token_templates {
        game_loop.register_token(script_name.clone(), template.clone());
    }

    // Shared storage for turn-start snapshots captured by CapturingAgent
    let shared_snapshots: Arc<Mutex<Vec<StateSnapshot>>> = Arc::new(Mutex::new(Vec::new()));
    let shared_covered_cards: Arc<Mutex<BTreeSet<String>>> = Arc::new(Mutex::new(BTreeSet::new()));
    let shared_mechanic_signals: Arc<Mutex<BTreeMap<String, usize>>> =
        Arc::new(Mutex::new(BTreeMap::new()));
    let shared_decisions: Arc<Mutex<Vec<DecisionRecord>>> = Arc::new(Mutex::new(Vec::new()));

    // Run game with fixed seed (for any engine-internal randomness)
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Setup: shuffle libraries with Java-compatible RNG so opening hands match
    // the Java Forge engine, then draw 7 cards per player.
    //
    // Java's flow in match.startGame():
    //   1. prepareAllZones() — builds libraries (no RNG)
    //   2. player.shuffle(null) for each player — Collections.shuffle(list, rng)
    //   3. drawStartingHand() — moves top 7 cards to hand (no RNG)
    //
    // The game_rng mirrors Java's MyRandom — same seed, same consumption order.
    // It's used for both shuffling and game-level random effects (e.g.
    // Aggregates.random() in DiscardEffect Mode$ Random). After shuffling,
    // its state matches Java's MyRandom post-shuffle, so subsequent random
    // effects produce identical results.
    let game_rng = Rc::new(RefCell::new({
        let mut r = JavaRandom::new(config.seed as i64);
        r.label = "game";
        r
    }));
    {
        let t_shuffle = Instant::now();
        let mut shuffle_rng = game_rng.borrow_mut();
        for &pid in &game.player_order.clone() {
            // Sort library by card name for deterministic pre-shuffle ordering,
            // matching Java's Match.preparePlayerZone which sorts after building
            // from ConcurrentHashMap-backed CardPool.
            let mut lib_cards: Vec<CardId> = game.cards_in_zone(ZoneType::Library, pid).to_vec();
            lib_cards.sort_by(|a, b| {
                game.cards[a.index()]
                    .card_name
                    .cmp(&game.cards[b.index()].card_name)
            });
            // Shuffle using the Java-compatible PRNG
            shuffle_rng.shuffle(&mut lib_cards);
            // Reverse so Java's index-0 "top" becomes Rust's last-element "top"
            // (Rust draws via pop(), Java draws via get(0))
            lib_cards.reverse();
            // Write back the shuffled order
            game.zone_mut(ZoneType::Library, pid).cards = lib_cards;
        }
        perf::record("run_with_data.shuffle_libraries", t_shuffle.elapsed());
    }

    // Match Java's determineFirstTurnPlayer() "coin flip".
    // Java calls Aggregates.random(game.getPlayers()) where game.getPlayers()
    // returns a PlayerCollection (extends FCollection<Player> implements List<Player>).
    // Since it implements List, Aggregates.random takes the List fast-path:
    //   src.get(MyRandom.getRandom().nextInt(len))
    // That's a single nextInt(numPlayers) call.
    // The result is overridden by DeterministicController.chooseStartingPlayer()
    // which always returns player 0 — we just need to consume the same RNG call.
    {
        let num_players = game.player_order.len() as i32;
        let _coin_flip = game_rng.borrow_mut().next_int(num_players);
    }
    for &pid in &game.player_order.clone() {
        game.draw_cards(pid, 7);
    }
    // Create a SEPARATE agent RNG seeded identically to Java's `new Random(seed)`.
    // This is distinct from the game RNG — both sides create a fresh Random(seed)
    // for agent decisions, ensuring the RNG state matches even though the game
    // RNG is consumed differently by each engine's internals.
    // IMPORTANT: Must be created BEFORE Leyline placement since Java's
    // chooseSaToActivateFromOpeningHand consumes this RNG.
    let agent_rng = Rc::new(RefCell::new({
        let mut r = JavaRandom::new(config.seed as i64);
        r.label = "agent";
        r
    }));

    // Leyline mechanic: cards with "MayEffectFromOpeningHand" in hand
    // may begin the game on the battlefield. Java's DeterministicController
    // calls ChoiceSpace.pickBool(rng) for each eligible card, consuming RNG.
    // We must match that consumption pattern for parity.
    for &pid in &game.player_order.clone() {
        let hand: Vec<forge_engine_core::ids::CardId> = game.cards_in_zone(forge_foundation::ZoneType::Hand, pid).to_vec();
        for card_id in hand {
            if game.card(card_id).get_keyword_cost("MayEffectFromOpeningHand").is_some() {
                // Java: ChoiceSpace.pickBool(rng) → rng.nextInt(2) == 1
                let place = agent_rng.borrow_mut().next_int(2) == 1;
                if place {
                    game.move_card(card_id, forge_foundation::ZoneType::Battlefield, pid);
                }
            }
        }
    }

    let parity_map = Arc::new(ParityCardMap::from_opening_state(&game));

    // Create deterministic agents — player 0 uses CapturingAgent to collect
    // turn-start snapshots (matching Java's GameEventTurnBegan timing).
    // Both agents share the same agent RNG so consumption order matches Java.
    // Both agents share the same game RNG so random effects match Java's MyRandom.
    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
        Box::new(CapturingAgent::new(
            p0,
            config.verbose,
            config.prefer_actions,
            Arc::clone(&shared_snapshots),
            Arc::clone(&shared_covered_cards),
            Arc::clone(&shared_mechanic_signals),
            Arc::clone(&shared_decisions),
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            Arc::clone(&parity_map),
            true,
        )),
        Box::new(CapturingAgent::new(
            p1,
            config.verbose,
            config.prefer_actions,
            Arc::clone(&shared_snapshots),
            Arc::clone(&shared_covered_cards),
            Arc::clone(&shared_mechanic_signals),
            Arc::clone(&shared_decisions),
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            Arc::clone(&parity_map),
            false,
        )),
    ];

    // Wire the Java-compatible RNG into the game loop so that effect-level
    // shuffles, coin flips, and dice rolls consume the same PRNG instance
    // as the agents, matching Java's single MyRandom consumption order.
    game_loop.game_rng = Box::new(crate::java_random::JavaGameRng(Rc::clone(&game_rng)));

    // Run turns — CapturingAgent captures turn-start snapshots automatically
    while !game.game_over && game.turn.turn_number <= config.max_turns {
        let t_turn = Instant::now();
        game_loop.run_turn(&mut game, &mut agents, &mut rng);
        perf::record("run_with_data.run_turn", t_turn.elapsed());
    }

    // Collect turn-start snapshots from the shared storage.
    let turn_snapshots = shared_snapshots.lock().unwrap();
    let snapshots: Vec<StateSnapshot> = turn_snapshots.clone();
    drop(turn_snapshots);
    let decisions: Vec<DecisionRecord> = shared_decisions.lock().unwrap().clone();
    let covered_cards: Vec<String> = shared_covered_cards
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect();
    let mechanic_signals: Vec<MechanicSignal> = shared_mechanic_signals
        .lock()
        .unwrap()
        .iter()
        .map(|(label, count)| MechanicSignal {
            label: label.clone(),
            count: *count,
        })
        .collect();

    perf::record("run_with_data.total", t_total.elapsed());
    Ok(GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        variant: config.variant.clone(),
        commanders: config.commanders.clone(),
        snapshots,
        decisions,
        covered_cards,
        mechanic_signals,
    })
}

/// Run the Rust engine with deterministic agents and collect per-phase snapshots.
/// Convenience wrapper that loads data fresh each call.
pub fn run_rust_only(config: &RunConfig) -> Result<GameTrace, String> {
    let data = load_data(config.cards_dir.as_deref(), config.verbose)?;
    run_with_data(config, &data)
}

fn extract_coverage_card(message: &str) -> Option<&str> {
    message
        .strip_prefix("Played land: ")
        .or_else(|| message.strip_prefix("Cast: "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn extract_mechanic_signal(message: &str) -> Option<String> {
    if message.starts_with("Illegal action") || message.starts_with("Card play failed") {
        return None;
    }
    if message.starts_with("Played land: ") || message.starts_with("Cast: ") {
        return None;
    }
    if let Some(rest) = message.strip_prefix("Trigger fired:") {
        let mode = extract_pipe_value(rest, "mode").unwrap_or("Unknown");
        let api = extract_pipe_value(rest, "api").unwrap_or("Unknown");
        return Some(format!("Trigger fired: mode={} | api={}", mode, api));
    }
    if let Some(rest) = message.strip_prefix("Effect resolved:") {
        let api = rest.split('|').next().map(str::trim).unwrap_or("Unknown");
        return Some(format!("Effect resolved: {}", api));
    }
    if let Some(rest) = message.strip_prefix("Activated ability:") {
        let api = rest.split('|').next().map(str::trim).unwrap_or("Unknown");
        return Some(format!("Activated ability: {}", api));
    }
    let interesting = [
        "Suspend:",
        "Foretold:",
        "Storm count:",
        "Storm copy:",
        "Replicate:",
        "Replicate copy:",
        "Cascade found:",
        "Cascade cast:",
        "Rebound:",
        "Revealed:",
        "Rolled a ",
    ];
    if interesting.iter().any(|p| message.starts_with(p)) {
        return Some(message.to_string());
    }
    None
}

fn extract_pipe_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split('|').find_map(|part| {
        let trimmed = part.trim();
        let (lhs, rhs) = trimmed.split_once('=')?;
        if lhs.trim().eq_ignore_ascii_case(key) {
            Some(rhs.trim())
        } else {
            None
        }
    })
}
