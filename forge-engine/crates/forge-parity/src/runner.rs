//! ParityRunner: orchestrates game execution and snapshot collection.
//!
//! Loads decks, sets up `GameState` + `GameLoop` with a fixed RNG seed,
//! captures a [`StateSnapshot`] after each phase, and collects them into a
//! [`GameTrace`].

use std::sync::{Arc, Mutex};

use forge_carddb::{CardDatabase, CardRules};
use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::card::{CardInstance, CardOtherPart};
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
use forge_engine_core::trigger::parse_trigger;
use forge_foundation::ZoneType;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::deterministic_agent::DeterministicAgent;
use crate::java_random::JavaRandom;
use crate::protocol::{GameTrace, StateSnapshot};
use crate::snapshot::snapshot_game;

// ── Preset Deck Lists ──────────────────────────────────────────────
// Mirrored from src-tauri/src/preset_decks.rs so this crate is self-contained.
// Each entry is (card_name, count).

const RED_BURN: &[(&str, usize)] = &[
    ("Mountain", 17),
    ("Lightning Bolt", 4),
    ("Shock", 4),
    ("Gray Ogre", 3),
    ("Hill Giant", 3),
    ("Guttersnipe", 3),
];

const GREEN_STOMPY: &[(&str, usize)] = &[
    ("Forest", 17),
    ("Giant Growth", 4),
    ("Grizzly Bears", 3),
    ("Centaur Courser", 2),
    ("Garruk's Companion", 3),
    ("Giant Spider", 2),
    ("Wall of Ice", 2),
    ("Craw Wurm", 2),
];

const WHITE_AGGRO: &[(&str, usize)] = &[
    ("Plains", 17),
    ("Savannah Lions", 4),
    ("White Knight", 3),
    ("Serra Angel", 3),
    ("Soul Warden", 3),
];

const BLACK_CONTROL: &[(&str, usize)] = &[
    ("Swamp", 17),
    ("Doom Blade", 4),
    ("Dark Ritual", 2),
    ("Hypnotic Specter", 3),
    ("Sengir Vampire", 2),
];

/// Resolve a preset deck name to a card list.
fn get_preset_deck(name: &str) -> Option<&'static [(&'static str, usize)]> {
    match name {
        "red_burn" => Some(RED_BURN),
        "green_stompy" => Some(GREEN_STOMPY),
        "white_aggro" => Some(WHITE_AGGRO),
        "black_control" => Some(BLACK_CONTROL),
        _ => None,
    }
}

/// All available preset deck IDs.
pub fn available_presets() -> Vec<&'static str> {
    vec!["red_burn", "green_stompy", "white_aggro", "black_control"]
}

// ── Card Instance Builder ──────────────────────────────────────────
// Replicates card_rules_to_instance from src-tauri/src/card_db.rs.

fn card_rules_to_instance(rules: &CardRules, owner: PlayerId) -> CardInstance {
    let face = &rules.main_part;
    let mut next_trigger_id = 0u32;

    let triggers: Vec<_> = face
        .triggers
        .iter()
        .filter_map(|raw| parse_trigger(raw, &mut next_trigger_id))
        .collect();

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

    card.triggers = triggers;
    card.svars = face.svars.clone();

    for raw in &face.static_abilities {
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

fn build_deck(
    game: &mut GameState,
    db: &CardDatabase,
    owner: PlayerId,
    deck: &[(&str, usize)],
) {
    for (name, count) in deck {
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
    /// Snapshot cached by `snapshot_state()`, pushed on `notify_turn_changed()`.
    pending_snapshot: Option<StateSnapshot>,
}

impl CapturingAgent {
    fn new(player_id: PlayerId, verbose: bool, shared: Arc<Mutex<Vec<StateSnapshot>>>) -> Self {
        Self {
            inner: DeterministicAgent::new(player_id, verbose),
            shared_snapshots: shared,
            pending_snapshot: None,
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
        // Cache the snapshot — it will be pushed when notify_turn_changed fires
        self.pending_snapshot = Some(snapshot_game(game));
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        self.inner.notify_turn_changed(active_player, turn_number);
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

    fn on_library_peek(
        &mut self,
        game: &GameState,
        cards: &[CardId],
    ) {
        self.inner.on_library_peek(game, cards);
    }

    fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId]) -> bool {
        self.inner.mulligan_decision(player, hand)
    }

    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> forge_engine_core::agent::MainPhaseAction {
        self.inner
            .choose_action(player, playable, tappable_lands, untappable_lands, activatable)
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

    fn choose_target_player(
        &mut self,
        player: PlayerId,
        valid: &[PlayerId],
    ) -> Option<PlayerId> {
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

    fn choose_discard(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        self.inner.choose_discard(player, hand, num)
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
        self.inner.choose_mode(player, descriptions, min, max, card_name)
    }

    fn choose_optional_trigger(&mut self, player: PlayerId, description: &str, card_name: Option<&str>) -> bool {
        self.inner.choose_optional_trigger(player, description, card_name)
    }

    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool> {
        self.inner.choose_land_or_spell(player)
    }

    fn notify(&mut self, message: &str) {
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
    pub verbose: bool,
}

/// Run the Rust engine with deterministic agents and collect per-phase snapshots.
pub fn run_rust_only(config: &RunConfig) -> Result<GameTrace, String> {
    // Load card database
    let cards_dir = config
        .cards_dir
        .as_deref()
        .unwrap_or("forge/forge-gui/res/cardsfolder");
    let cards_path = std::path::Path::new(cards_dir);

    if !cards_path.exists() {
        return Err(format!(
            "Cards directory not found: {}. Set --cards-dir to the Forge cardsfolder path.",
            cards_dir,
        ));
    }

    eprintln!("[parity] Loading cards from {:?} ...", cards_path);
    let (db, result) = CardDatabase::load_from_directory(cards_path);
    eprintln!(
        "[parity] Loaded {} cards ({} failed)",
        result.loaded, result.failed
    );

    // Resolve deck lists
    let deck1_list = get_preset_deck(&config.deck1).ok_or_else(|| {
        format!(
            "Unknown deck '{}'. Available: {:?}",
            config.deck1,
            available_presets()
        )
    })?;
    let deck2_list = get_preset_deck(&config.deck2).ok_or_else(|| {
        format!(
            "Unknown deck '{}'. Available: {:?}",
            config.deck2,
            available_presets()
        )
    })?;

    // Set up game
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);
    let mut game = GameState::new(&["Player1", "Player2"], 20);

    build_deck(&mut game, &db, p0, deck1_list);
    build_deck(&mut game, &db, p1, deck2_list);

    let mut game_loop = GameLoop::new(2);

    // Load token templates
    let token_dir_path = cards_path
        .parent()
        .map(|p| p.join("tokenscripts"))
        .unwrap_or_default();
    if token_dir_path.exists() {
        eprintln!("[parity] Loading token scripts from {:?} ...", token_dir_path);
        let (token_db, token_result) = CardDatabase::load_from_directory(&token_dir_path);
        eprintln!(
            "[parity] Loaded {} token scripts",
            token_result.loaded
        );
        for (script_name, rules) in token_db.iter() {
            let template = card_rules_to_instance(rules, PlayerId(0));
            game_loop.register_token(script_name.clone(), template);
        }
    }

    // Shared storage for turn-start snapshots captured by CapturingAgent
    let shared_snapshots: Arc<Mutex<Vec<StateSnapshot>>> = Arc::new(Mutex::new(Vec::new()));

    // Create deterministic agents — player 0 uses CapturingAgent to collect
    // turn-start snapshots (matching Java's GameEventTurnBegan timing).
    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
        Box::new(CapturingAgent::new(p0, config.verbose, Arc::clone(&shared_snapshots))),
        Box::new(DeterministicAgent::new(p1, config.verbose)),
    ];

    // Run game with fixed seed
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Setup: shuffle libraries with Java-compatible RNG so opening hands match
    // the Java Forge engine, then draw 7 cards per player.
    //
    // Java's flow in match.startGame():
    //   1. prepareAllZones() — builds libraries (no RNG)
    //   2. player.shuffle(null) for each player — Collections.shuffle(list, rng)
    //   3. drawStartingHand() — moves top 7 cards to hand (no RNG)
    {
        let mut java_rng = JavaRandom::new(config.seed as i64);
        for &pid in &game.player_order.clone() {
            // Sort library by card name for deterministic pre-shuffle ordering,
            // matching Java's Match.preparePlayerZone which sorts after building
            // from ConcurrentHashMap-backed CardPool.
            let mut lib_cards: Vec<CardId> =
                game.cards_in_zone(ZoneType::Library, pid).to_vec();
            lib_cards.sort_by(|a, b| {
                game.cards[a.index()].card_name.cmp(&game.cards[b.index()].card_name)
            });
            // Shuffle using the Java-compatible PRNG
            java_rng.shuffle(&mut lib_cards);
            // Reverse so Java's index-0 "top" becomes Rust's last-element "top"
            // (Rust draws via pop(), Java draws via get(0))
            lib_cards.reverse();
            // Write back the shuffled order
            game.zone_mut(ZoneType::Library, pid).cards = lib_cards;
        }
        for &pid in &game.player_order.clone() {
            game.draw_cards(pid, 7);
        }
    }

    // Run turns — CapturingAgent captures turn-start snapshots automatically
    while !game.game_over && game.turn.turn_number <= config.max_turns {
        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    // Collect turn-start snapshots from the shared storage.
    // No initial pre-game snapshot — Java only emits on GameEventTurnPhase(UNTAP),
    // so Rust[0] = T1 start should align with Java[0] = T1 start.
    // No final snapshot — the final game-over state is captured at different
    // timing between Rust (Cleanup phase) and Java (Untap phase after turn limit),
    // so it can't be meaningfully compared.
    let turn_snapshots = shared_snapshots.lock().unwrap();
    let snapshots: Vec<StateSnapshot> = turn_snapshots.clone();
    drop(turn_snapshots);

    Ok(GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots,
    })
}
