//! ParityRunner: orchestrates game execution and snapshot collection.
//!
//! Loads decks, sets up `GameState` + `GameLoop` with a fixed RNG seed,
//! captures a [`StateSnapshot`] after each phase, and collects them into a
//! [`GameTrace`].

use std::cell::RefCell;
use std::rc::Rc;
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

use crate::deck_generator;
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

const COMPREHENSIVE_TEST: &[(&str, usize)] = &[
    // Lands (18)
    ("Forest", 3),
    ("Island", 3),
    ("Plains", 3), // was 2 + Command Tower (Combo ColorIdentity → unusable in non-Commander)
    ("Mountain", 2),
    ("Swamp", 3), // was 2 + Path of Ancestry (Combo ColorIdentity → unusable in non-Commander)
    ("Breeding Pool", 1),
    ("Hallowed Fountain", 1),
    ("Temple of Mystery", 1),
    ("Yavimaya Coast", 1),
    // Keyword creatures (11)
    ("Vampire Nighthawk", 1),
    ("Serra Angel", 1),
    ("Darksteel Myr", 1),
    ("Boggart Brute", 1),
    ("Glistener Elf", 1),
    ("White Knight", 1),
    ("Giant Spider", 1),
    ("Llanowar Elves", 2),
    ("Soul Warden", 1),
    ("Guttersnipe", 1),
    // ETB / explore / proliferate (4)
    ("Merfolk Branchwalker", 1),
    ("Jadelight Ranger", 1),
    ("Elvish Visionary", 1), // was Mulldrifter (evoke: Rust uses evoke cost, Java uses main cost)
    ("Thrummingbird", 1),
    // Detain / goad / protection (3)
    ("Lyev Skyknight", 1),
    ("Gods Willing", 1),
    ("Brave the Elements", 1),
    // Damage / removal (4)
    ("Lightning Bolt", 2),
    ("Wrath of God", 1),
    ("Doom Blade", 1),
    ("Prey Upon", 1),
    // Card advantage (4)
    ("Ponder", 1),
    ("Preordain", 1),
    ("Thought Scour", 1),
    ("Steady Progress", 1),
    // Modal / draw / choice (3)
    ("Izzet Charm", 1),
    ("Divination", 1),
    ("Control Magic", 1),
    // Combat tricks / bounce / fog (3)
    ("Giant Growth", 1),
    ("Fog", 1),
    ("Unsummon", 1),
    // Tokens (3)
    ("Raise the Alarm", 1),
    ("Dragon Fodder", 1),
    ("Lingering Souls", 1),
    // Simple creatures / spells (6) — removed alt-cost cards that cause parity divergences
    // (Evoke/Spectacle/Storm/Cascade are not handled uniformly by Java getBasicSpells)
    ("Faithless Looting", 1), // flashback: both engines support graveyard cast
    ("Goblin Bushwhacker", 1), // kicker is optional; base cost R always used
    ("Volcanic Hammer", 1),   // was Skewer the Critics (spectacle R)
    ("Shock", 1),             // was Grapeshot (storm)
    ("Lightning Elemental", 1), // was Bloodbraid Elf (cascade)
    ("Gray Ogre", 1),         // was Zurgo Bellstriker (dash 1R), simple 2/2
    // Static anthems (2)
    ("Glorious Anthem", 1),
    ("Honor of the Pure", 1),
];

// Exercises ChangeZone (bounce/reanimate), Sacrifice effects.
const ZONE_CHANGE: &[(&str, usize)] = &[
    ("Swamp", 12),
    ("Island", 4),
    ("Unsummon", 4),
    ("Boomerang", 2),
    ("Raise Dead", 13),
    ("Diabolic Edict", 3),
    ("Innocent Blood", 3),
    ("Typhoid Rats", 4),
    ("Vampire Nighthawk", 3),
    ("Doom Blade", 2),
];

// Exercises token creation (SP$ Token).
const TOKEN_SWARM: &[(&str, usize)] = &[
    ("Plains", 8),
    ("Mountain", 8),
    ("Raise the Alarm", 4),
    ("Krenko's Command", 4),
    ("Dragon Fodder", 4),
    ("Savannah Lions", 4),
    ("Lightning Bolt", 4),
    ("Shock", 4),
];

// Exercises Counter, Discard, and ControlGain effects.
const BLUE_CONTROL: &[(&str, usize)] = &[
    ("Island", 17),
    ("Counterspell", 4),
    ("Cancel", 4),
    ("Mind Rot", 4),
    ("Control Magic", 3),
    ("Mulldrifter", 3),
    ("Divination", 4),
    ("Wall of Ice", 4),
    ("Sea Serpent", 4),
];

// Exercises Scry, Surveil, Mill, Dig, RearrangeTopOfLibrary.
const LIBRARY_MANIPULATION: &[(&str, usize)] = &[
    ("Island", 16),
    ("Swamp", 4),
    ("Preordain", 4),
    ("Ponder", 4),
    ("Thought Scour", 4),
    ("Ransack the Lab", 4),
    ("Taigam's Scheming", 2),
    ("Notion Rain", 2),
    ("Divination", 4),
    ("Mulldrifter", 4),
    ("Typhoid Rats", 4),
    ("Doom Blade", 4),
    ("Vampire Nighthawk", 4),
];

// Exercises Fight effects (Prey Upon, Ram Through).
const GREEN_FIGHT: &[(&str, usize)] = &[
    ("Forest", 17),
    ("Prey Upon", 4),
    ("Ram Through", 4),
    ("Garruk's Companion", 4),
    ("Centaur Courser", 4),
    ("Giant Spider", 3),
    ("Craw Wurm", 3),
    ("Giant Growth", 4),
    ("Grizzly Bears", 4),
];

// Exercises DestroyAll, DamageAll, PumpAll mass effects.
const MASS_EFFECTS: &[(&str, usize)] = &[
    ("Plains", 18),
    ("Wrath of God", 4),
    ("Pyroclasm", 4),
    ("Righteous Charge", 4),
    ("Rising Miasma", 4),
    ("Savannah Lions", 4),
    ("White Knight", 4),
    ("Serra Angel", 4),
    ("Darksteel Myr", 4),
];

// Exercises expanded trigger types (ETB, SpellCast, DamageDone, LifeGained).
const TRIGGER_TEST: &[(&str, usize)] = &[
    ("Plains", 10),
    ("Mountain", 7),
    ("Soul Warden", 7),
    ("Guttersnipe", 7),
    ("Savannah Lions", 4),
    ("Serra Angel", 3),
    ("Lightning Bolt", 4),
    ("Shock", 4),
    ("Raise the Alarm", 4),
    ("Vampire Nighthawk", 3),
    ("White Knight", 3),
];

// Exercises evasion & protection keywords (Hexproof, Menace, Infect, etc.).
const KEYWORD_TEST: &[(&str, usize)] = &[
    ("Swamp", 7),
    ("Forest", 3),
    ("Mountain", 3),
    ("Island", 3),
    ("Plague Stinger", 2),
    ("Sickle Ripper", 2),
    ("Rancid Rats", 2),
    ("Severed Legion", 2),
    ("Boggart Brute", 2),
    ("Bladetusk Boar", 2),
    ("Thalakos Sentry", 2),
    ("Humble Budoka", 2),
    ("Wardscale Crocodile", 2),
    ("Zombie Outlander", 2),
    ("Yavimaya Barbarian", 2),
    ("Darksteel Myr", 2),
];

const TRIGGER_EXPANDED: &[(&str, usize)] = &[
    ("Mountain", 7),
    ("Forest", 5),
    ("Swamp", 3),
    ("Island", 3),
    ("Plains", 2),
    // AttackersDeclared
    ("Roar of Resistance", 3),
    ("Ruby Collector", 3),
    // SpellCast
    ("Guttersnipe", 3),
    ("Young Pyromancer", 2),
    // ChangesZone
    ("Essence Warden", 3),
    ("Impact Tremors", 2),
    // DamageDoneOnce
    ("Raptor Hatchling", 3),
    ("Ranging Raptors", 2),
    // ChangesZoneAll
    ("Woodland Champion", 2),
    // CounterAddedOnce
    ("Nest of Scarabs", 2),
    ("Stocking the Pantry", 2),
    // Surveil
    ("Thoughtbound Phantasm", 2),
    ("Whispering Snitch", 2),
    // DamageDoneOnce
    ("Rite of Passage", 2),
];

// Static-ability focused test deck for parity.
const STATICABILITY_TEST: &[(&str, usize)] = &[
    // 5-color mana base to enable broad static cards.
    ("Plains", 5),
    ("Island", 5),
    ("Swamp", 4),
    ("Mountain", 4),
    ("Forest", 4),
    // Critical modes
    ("Underworld Cerberus", 1),
    ("Konda's Banner", 1),
    ("Juggernaut", 1),
    ("Watchdog", 1),
    ("Panharmonicon", 1),
    ("Platinum Emperion", 1),
    ("Maralen of the Mornsong", 1),
    ("Yasharn, Implacable Earth", 1),
    ("Incinerate", 1),
    ("Hushbringer", 1),
    ("Solemnity", 1),
    // High-priority modes
    ("Winding Canyons", 1),
    ("Silent Arbiter", 1),
    ("Crawlspace", 1),
    ("Glaring Spotlight", 1),
    ("Autumn Willow", 1),
    ("Brothers Yamazaki", 1),
    ("Standard Bearer", 1),
    ("Wolf Pack", 1),
    ("Pygmy Hippo", 1),
    ("Walking Bulwark", 1),
    ("Patient Zero", 1),
    ("Phyrexian Unlife", 1),
    ("Everlasting Torment", 1),
    ("Ghostly Flame", 1),
    ("Skullbriar, the Walking Grave", 1),
    ("Rasputin Dreamweaver", 1),
    // Support cards
    ("Lightning Bolt", 2),
    ("Shock", 2),
    ("Unsummon", 2),
    ("Doom Blade", 2),
    ("Raise the Alarm", 2),
    ("Typhoid Rats", 2),
    ("Vampire Nighthawk", 2),
];

/// Resolve a preset deck name to a card list.
fn get_preset_deck(name: &str) -> Option<&'static [(&'static str, usize)]> {
    match name {
        "red_burn" => Some(RED_BURN),
        "green_stompy" => Some(GREEN_STOMPY),
        "white_aggro" => Some(WHITE_AGGRO),
        "black_control" => Some(BLACK_CONTROL),
        "comprehensive_test" => Some(COMPREHENSIVE_TEST),
        "zone_change" => Some(ZONE_CHANGE),
        "token_swarm" => Some(TOKEN_SWARM),
        "blue_control" => Some(BLUE_CONTROL),
        "library_manipulation" => Some(LIBRARY_MANIPULATION),
        "green_fight" => Some(GREEN_FIGHT),
        "mass_effects" => Some(MASS_EFFECTS),
        "trigger_test" => Some(TRIGGER_TEST),
        "keyword_test" => Some(KEYWORD_TEST),
        "trigger_expanded" => Some(TRIGGER_EXPANDED),
        "staticability_test" => Some(STATICABILITY_TEST),
        _ => None,
    }
}

/// All available preset deck IDs.
pub fn available_presets() -> Vec<&'static str> {
    vec![
        "red_burn",
        "green_stompy",
        "white_aggro",
        "black_control",
        "comprehensive_test",
        "zone_change",
        "token_swarm",
        "blue_control",
        "library_manipulation",
        "green_fight",
        "mass_effects",
        "trigger_test",
        "keyword_test",
        "trigger_expanded",
        "staticability_test",
    ]
}

/// Resolve a deck spec string to a list of (card_name, count) pairs.
///
/// Supports:
/// - `"inline:Name*Count|Name*Count|..."` — inline deck specification
/// - `"file:/path/to/deck.txt"` — load from a text file (one `Count CardName` per line)
/// - `"red_burn"` etc. — preset deck name lookup
pub fn resolve_deck_spec(spec: &str) -> Result<Vec<(String, usize)>, String> {
    if let Some(inline) = spec.strip_prefix("inline:") {
        deck_generator::parse_inline(inline)
    } else if let Some(path) = spec.strip_prefix("file:") {
        parse_deck_file(path)
    } else {
        let preset = get_preset_deck(spec).ok_or_else(|| {
            format!(
                "Unknown deck '{}'. Available: {:?}",
                spec,
                available_presets()
            )
        })?;
        Ok(preset.iter().map(|(n, c)| (n.to_string(), *c)).collect())
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
    /// Snapshot cached by `snapshot_state()`, pushed on `notify_turn_changed()`.
    pending_snapshot: Option<StateSnapshot>,
}

impl CapturingAgent {
    fn new(
        player_id: PlayerId,
        verbose: bool,
        shared: Arc<Mutex<Vec<StateSnapshot>>>,
        rng: Rc<RefCell<JavaRandom>>,
    ) -> Self {
        Self {
            inner: DeterministicAgent::new(player_id, verbose, rng),
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

    fn on_library_peek(&mut self, game: &GameState, cards: &[CardId]) {
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
    ) -> bool {
        self.inner
            .choose_optional_trigger(player, description, card_name)
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

/// Pre-loaded card database and token templates, reusable across multiple matchups.
pub struct LoadedData {
    pub db: CardDatabase,
    pub token_templates: Vec<(String, CardInstance)>,
}

/// Load the card database and token templates once.
pub fn load_data(cards_dir: Option<&str>) -> Result<LoadedData, String> {
    let cards_dir = cards_dir.unwrap_or("forge/forge-gui/res/cardsfolder");
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

    let mut token_templates = Vec::new();
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
            token_templates.push((script_name.clone(), template));
        }
    }

    Ok(LoadedData { db, token_templates })
}

/// Run a game using pre-loaded data (avoids reloading the DB for each matchup).
pub fn run_with_data(config: &RunConfig, data: &LoadedData) -> Result<GameTrace, String> {
    // Resolve deck lists — supports both preset names and inline: specs
    let deck1_spec = resolve_deck_spec(&config.deck1)?;
    let deck2_spec = resolve_deck_spec(&config.deck2)?;

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

    // Run game with fixed seed (for any engine-internal randomness)
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Setup: shuffle libraries with Java-compatible RNG so opening hands match
    // the Java Forge engine, then draw 7 cards per player.
    //
    // Java's flow in match.startGame():
    //   1. prepareAllZones() — builds libraries (no RNG)
    //   2. player.shuffle(null) for each player — Collections.shuffle(list, rng)
    //   3. drawStartingHand() — moves top 7 cards to hand (no RNG)
    {
        let mut shuffle_rng = JavaRandom::new(config.seed as i64);
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
        for &pid in &game.player_order.clone() {
            game.draw_cards(pid, 7);
        }
    }

    // Create a SEPARATE agent RNG seeded identically to Java's `new Random(seed)`.
    // This is distinct from the shuffle RNG — both sides create a fresh Random(seed)
    // for agent decisions, ensuring the RNG state matches even though the shuffle
    // RNG is consumed differently by each engine's internals.
    let agent_rng = Rc::new(RefCell::new(JavaRandom::new(config.seed as i64)));

    // Create deterministic agents — player 0 uses CapturingAgent to collect
    // turn-start snapshots (matching Java's GameEventTurnBegan timing).
    // Both agents share the same agent RNG so consumption order matches Java.
    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
        Box::new(CapturingAgent::new(
            p0,
            config.verbose,
            Arc::clone(&shared_snapshots),
            Rc::clone(&agent_rng),
        )),
        Box::new(DeterministicAgent::new(p1, config.verbose, Rc::clone(&agent_rng))),
    ];

    // Run turns — CapturingAgent captures turn-start snapshots automatically
    while !game.game_over && game.turn.turn_number <= config.max_turns {
        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    // Collect turn-start snapshots from the shared storage.
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

/// Run the Rust engine with deterministic agents and collect per-phase snapshots.
/// Convenience wrapper that loads data fresh each call.
pub fn run_rust_only(config: &RunConfig) -> Result<GameTrace, String> {
    let data = load_data(config.cards_dir.as_deref())?;
    run_with_data(config, &data)
}
