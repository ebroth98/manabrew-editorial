//! ParityRunner: orchestrates game execution and snapshot collection.
//!
//! Loads decks, sets up `GameState` + `GameLoop` with a fixed RNG seed,
//! captures a [`StateSnapshot`] after each phase, and collects them into a
//! [`GameTrace`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use forge_carddb::{CardDatabase, CardRules};
use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::card::{CardInstance, CardOtherPart};
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::ability::activated::parse_activated_ability;
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
use forge_engine_core::trigger::parse_trigger;
use forge_foundation::ZoneType;
use rand::rngs::StdRng;
use rand::SeedableRng;

use serde::Deserialize;

use crate::deck_generator;
use crate::deterministic_agent::DeterministicAgent;
use crate::java_random::JavaRandom;
use crate::protocol::{GameTrace, MechanicSignal, StateSnapshot};
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
        let (count_str, name) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("Line {}: expected 'Count CardName', got '{}'", line_num + 1, line))?;
        let count: usize = count_str
            .trim()
            .parse()
            .map_err(|_| format!("Line {}: invalid count '{}' in '{}'", line_num + 1, count_str, line))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("Line {}: empty card name in '{}'", line_num + 1, line));
        }
        deck.push((name.to_string(), count));
    }
    if deck.is_empty() {
        return Err(format!("Deck file '{}' contains no cards", path));
    }
    Ok(deck)
}

// ── Card Instance Builder ──────────────────────────────────────────
// Replicates card_rules_to_instance from src-tauri/src/card_db.rs.
// IMPORTANT: Keep in sync with card_db.rs when adding new keyword/trigger logic.

/// Parse `Mode$ AlternativeCost | Cost$ GainLife<N/...> | IsPresent$ ...` from a
/// static ability raw string and return `Some("AltCostGainLife:N:condition")` keyword.
fn parse_gainlife_alt_cost_keyword(raw: &str) -> Option<String> {
    if !raw.contains("AlternativeCost") {
        return None;
    }
    let life_amount = raw.split('|').find_map(|part| {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("Cost$") {
            let cost = rest.trim();
            if let Some(inner) = cost.strip_prefix("GainLife<").and_then(|s| s.split('>').next()) {
                let n = inner.split('/').next().and_then(|s| s.trim().parse::<i32>().ok())?;
                return Some(n);
            }
        }
        None
    })?;
    let condition = raw.split('|').find_map(|part| {
        let p = part.trim();
        p.strip_prefix("IsPresent$").map(|s| s.trim().to_string())
    }).unwrap_or_default();
    Some(format!("AltCostGainLife:{}:{}", life_amount, condition))
}

fn card_rules_to_instance(rules: &CardRules, owner: PlayerId) -> CardInstance {
    let face = &rules.main_part;
    let mut next_trigger_id = 0u32;

    let mut triggers: Vec<_> = Vec::new();
    let mut spell_cast_or_copy_raw: Vec<String> = Vec::new();
    for raw in &face.triggers {
        let result = parse_trigger(raw, &mut next_trigger_id);
        if let Some(trig) = result {
            triggers.push(trig);
            if raw.contains("Mode$ SpellCastOrCopy") {
                spell_cast_or_copy_raw.push(raw.clone());
            }
        }
    }
    for raw in &spell_cast_or_copy_raw {
        let converted = raw.replace("Mode$ SpellCastOrCopy", "Mode$ SpellCopied");
        if let Some(trig) = parse_trigger(&converted, &mut next_trigger_id) {
            triggers.push(trig);
        }
    }

    // Auto-generate keyword triggers (e.g. Prowess)
    for kw in &face.keywords {
        if kw == "Prowess" {
            let raw = "Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | Execute$ TrigProwess | TriggerZones$ Battlefield | TriggerDescription$ Prowess";
            if let Some(mut trig) = parse_trigger(raw, &mut next_trigger_id) {
                trig.execute = "TrigProwess".to_string();
                triggers.push(trig);
            }
        }
    }

    let mut card = CardInstance::new(
        CardId(0),
        face.name.clone(),
        owner,
        face.type_line.clone(),
        face.mana_cost.clone(),
        face.resolved_color(),
        face.int_power,
        face.int_toughness,
        face.keywords.clone(),
        face.abilities.clone(),
    );

    // Auto-generate intrinsic mana abilities for basic land subtypes.
    const SUBTYPE_MANA: &[(&str, &str, &str)] = &[
        ("Plains", "W", "Add {W}."),
        ("Island", "U", "Add {U}."),
        ("Swamp", "B", "Add {B}."),
        ("Mountain", "R", "Add {R}."),
        ("Forest", "G", "Add {G}."),
    ];
    for &(subtype, letter, desc) in SUBTYPE_MANA {
        if card.type_line.has_subtype(subtype) {
            let already_produces = card.activated_abilities.iter().any(|ab| {
                ab.is_mana_ability && ab.params.get("Produced").map_or(false, |p| p == letter)
            });
            if !already_produces {
                let raw = format!(
                    "AB$ Mana | Cost$ T | Produced$ {} | SpellDescription$ {}",
                    letter, desc
                );
                let idx = card.abilities.len();
                card.abilities.push(raw.clone());
                if let Some(ab) = parse_activated_ability(&raw, idx) {
                    card.activated_abilities.push(ab);
                }
            }
        }
    }

    card.triggers = triggers;
    card.svars = face.svars.clone();

    // Inject Prowess SVar
    if face.keywords.iter().any(|k| k == "Prowess") && !card.svars.contains_key("TrigProwess") {
        card.svars.insert(
            "TrigProwess".to_string(),
            "DB$ Pump | Defined$ Self | NumAtt$ 1 | NumDef$ 1".to_string(),
        );
    }

    for raw in &face.static_abilities {
        // Convert Mode$ AlternativeCost | Cost$ GainLife<N> to keyword for runtime detection.
        if let Some(kw) = parse_gainlife_alt_cost_keyword(raw) {
            card.keywords.push(kw);
        }
        let prefixed = format!("S$ {}", raw);
        if let Some(sa) = parse_static_ability(&prefixed) {
            card.static_abilities.push(sa);
        }
    }

    for raw in &face.replacements {
        let prefixed = format!("R$ {}", raw);
        if let Some(re) = parse_replacement_effect(&prefixed) {
            card.replacement_effects.push(re);
        }
    }

    // Double-faced cards
    if rules.split_type.is_dual_faced() {
        if let Some(ref back_face) = rules.other_part {
            let mut back_trigger_id = 0u32;
            let back_triggers: Vec<_> = back_face
                .triggers
                .iter()
                .filter_map(|raw| parse_trigger(raw, &mut back_trigger_id))
                .collect();

            card.other_part = Some(CardOtherPart {
                name: back_face.name.clone(),
                type_line: back_face.type_line.clone(),
                mana_cost: back_face.mana_cost.clone(),
                color: back_face.resolved_color(),
                base_power: back_face.int_power,
                base_toughness: back_face.int_toughness,
                keywords: back_face.keywords.clone(),
                abilities: back_face.abilities.clone(),
                triggers: back_triggers,
                svars: back_face.svars.clone(),
            });
        }
    }

    card
}

/// Build a deck from a resolved spec. Used by inline/fuzz decks and presets.
fn build_deck_from_spec(
    game: &mut GameState,
    db: &CardDatabase,
    owner: PlayerId,
    spec: &[(String, usize)],
) {
    for (name, count) in spec {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let card = card_rules_to_instance(rules, owner);
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => eprintln!("[parity] Unknown card '{}' — skipped", name),
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
    inner: DeterministicAgent,
    /// Shared snapshot storage — collected after the game ends.
    shared_snapshots: Arc<Mutex<Vec<StateSnapshot>>>,
    /// Shared coverage storage — card names played/cast at least once.
    shared_covered_cards: Arc<Mutex<BTreeSet<String>>>,
    /// Shared low-effort mechanic signals (notify message buckets).
    shared_mechanic_signals: Arc<Mutex<BTreeMap<String, usize>>>,
    /// Snapshot cached by `snapshot_state()`, pushed on `notify_turn_changed()`.
    pending_snapshot: Option<StateSnapshot>,
    /// If true, capture snapshots at turn start.
    capture_snapshots: bool,
}

impl CapturingAgent {
    fn new(
        player_id: PlayerId,
        verbose: bool,
        prefer_actions: bool,
        shared: Arc<Mutex<Vec<StateSnapshot>>>,
        covered: Arc<Mutex<BTreeSet<String>>>,
        mechanics: Arc<Mutex<BTreeMap<String, usize>>>,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        capture_snapshots: bool,
    ) -> Self {
        Self {
            inner: DeterministicAgent::new(player_id, verbose, rng, game_rng, prefer_actions),
            shared_snapshots: shared,
            shared_covered_cards: covered,
            shared_mechanic_signals: mechanics,
            pending_snapshot: None,
            capture_snapshots,
        }
    }
}

impl PlayerAgent for CapturingAgent {
    fn snapshot_state(
        &mut self,
        game: &GameState,
        mana_pools: &[forge_engine_core::mana::ManaPool],
    ) {
        self.inner.snapshot_state(game, mana_pools);
        if !self.capture_snapshots {
            return;
        }
        // Cache the snapshot — it will be pushed when notify_turn_changed fires
        self.pending_snapshot = Some(snapshot_game(game));
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        self.inner.notify_turn_changed(active_player, turn_number);
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
        self.inner.notify_phase_changed(phase);
    }

    fn on_library_peek(&mut self, game: &GameState, cards: &[CardId]) {
        self.inner.on_library_peek(game, cards);
    }

    fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId], mulligan_count: u32) -> bool {
        self.inner.mulligan_decision(player, hand, mulligan_count)
    }

    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> forge_engine_core::agent::MainPhaseAction {
        self.inner.choose_action(
            player,
            playable,
            tappable_lands,
            untappable_lands,
            activatable,
        )
    }

    fn choose_attackers(&mut self, player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        self.inner.choose_attackers(player, available)
    }

    fn choose_blockers(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        self.inner
            .choose_blockers(player, attackers, available_blockers)
    }

    fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        self.inner.choose_target_player(player, valid)
    }

    fn choose_target_card(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        self.inner.choose_target_card(player, valid)
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        zone: ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        self.inner.choose_target_card_from_zone(player, zone, valid)
    }

    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> forge_engine_core::agent::TargetChoice {
        self.inner
            .choose_target_any(player, valid_players, valid_cards)
    }

    fn choose_sacrifice(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        self.inner.choose_sacrifice(player, valid)
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
        self.inner.choose_dig(player, valid, max, optional)
    }

    fn choose_reorder_library(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        self.inner.choose_reorder_library(player, cards)
    }

    fn choose_may_shuffle(&mut self, player: PlayerId) -> bool {
        self.inner.choose_may_shuffle(player)
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

    fn choose_optional_trigger(
        &mut self,
        player: PlayerId,
        description: &str,
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        self.inner
            .choose_optional_trigger(player, description, card_name, api)
    }

    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool> {
        self.inner.choose_land_or_spell(player)
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
}

/// Pre-loaded card database and token templates, reusable across multiple matchups.
pub struct LoadedData {
    pub db: CardDatabase,
    pub token_templates: Vec<(String, CardInstance)>,
}

/// Load the card database and token templates once.
pub fn load_data(cards_dir: Option<&str>, verbose: bool) -> Result<LoadedData, String> {
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
    let (db, result) = CardDatabase::load_from_directory(cards_path);
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
            eprintln!("[parity] Loading token scripts from {:?} ...", token_dir_path);
        }
        let (token_db, token_result) = CardDatabase::load_from_directory(&token_dir_path);
        if verbose {
            eprintln!(
                "[parity] Loaded {} token scripts",
                token_result.loaded
            );
        }
        for (script_name, rules) in token_db.iter() {
            let template = card_rules_to_instance(rules, PlayerId(0));
            token_templates.push((script_name.clone(), template));
        }
    }

    Ok(LoadedData { db, token_templates })
}

/// Run a game using pre-loaded data (avoids reloading the DB for each matchup).
pub fn run_with_data(config: &RunConfig, data: &LoadedData) -> Result<GameTrace, String> {
    // Resolve deck lists — supports preset names, inline: specs, and file: specs
    let decks_dir = config.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let deck1_spec = resolve_deck_spec(&config.deck1, decks_dir)?;
    let deck2_spec = resolve_deck_spec(&config.deck2, decks_dir)?;

    // Set up game
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);
    let mut game = GameState::new(&["Player1", "Player2"], 20);

    build_deck_from_spec(&mut game, &data.db, p0, &deck1_spec);
    build_deck_from_spec(&mut game, &data.db, p1, &deck2_spec);

    let mut game_loop = GameLoop::new(2);

    // Register token templates
    for (script_name, template) in &data.token_templates {
        game_loop.register_token(script_name.clone(), template.clone());
    }

    // Shared storage for turn-start snapshots captured by CapturingAgent
    let shared_snapshots: Arc<Mutex<Vec<StateSnapshot>>> = Arc::new(Mutex::new(Vec::new()));
    let shared_covered_cards: Arc<Mutex<BTreeSet<String>>> =
        Arc::new(Mutex::new(BTreeSet::new()));
    let shared_mechanic_signals: Arc<Mutex<BTreeMap<String, usize>>> =
        Arc::new(Mutex::new(BTreeMap::new()));

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
    let game_rng = Rc::new(RefCell::new(JavaRandom::new(config.seed as i64)));
    {
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
    }

    // Match Java's determineFirstTurnPlayer() "coin flip".
    // Java calls Aggregates.random(game.getPlayers()) which does nextInt(numPlayers)
    // on MyRandom to pick who goes first. The result is then overridden by
    // DeterministicController.chooseStartingPlayer() which always returns player 0.
    // We must consume the same RNG call to keep game_rng in sync, but ignore the result.
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
    let agent_rng = Rc::new(RefCell::new(JavaRandom::new(config.seed as i64)));

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
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            true,
        )),
        Box::new(CapturingAgent::new(
            p1,
            config.verbose,
            config.prefer_actions,
            Arc::clone(&shared_snapshots),
            Arc::clone(&shared_covered_cards),
            Arc::clone(&shared_mechanic_signals),
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            false,
        )),
    ];

    // Run turns — CapturingAgent captures turn-start snapshots automatically
    while !game.game_over && game.turn.turn_number <= config.max_turns {
        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    // Collect turn-start snapshots from the shared storage.
    let turn_snapshots = shared_snapshots.lock().unwrap();
    let snapshots: Vec<StateSnapshot> = turn_snapshots.clone();
    drop(turn_snapshots);
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

    Ok(GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots,
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
