//! A deterministic-but-random [`PlayerAgent`] for reproducible game traces.
//!
//! Uses a hybrid RNG strategy to stay in lockstep with the Java harness:
//! - **RNG for 4 core decisions**: play choice, attackers, blockers, targeting.
//! - **Fixed values for everything else**: scry, surveil, discard, modes, etc.
//!   use trait defaults (first option, keep all on top, always accept).
//!
//! This avoids RNG desync caused by Java and Rust calling non-core callbacks
//! (confirmAction, arrangeForScry, etc.) at different times or frequencies.
//! Both engines consume the RNG in the same order for the core decisions,
//! producing identical game traces for the same seed.
//!
//! Strategy:
//! - Always keep opening hand (mulligan changes state too much)
//! - Main phase: randomly pick from playable cards / activatable abilities or pass
//! - Attackers: per-creature coin flip (rng.nextInt(2))
//! - Blockers: per-blocker random attacker assignment
//! - Targets: random from sorted valid options
//! - All other decisions: trait defaults (first option, no RNG consumed)

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use forge_engine_core::agent::{
    BinaryChoiceKind, MainPhaseAction, PlayCardMode, PlayOption, PlayerAgent, TargetChoice,
};
use forge_engine_core::combat::DefenderId;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_engine_core::spellability::AlternativeCost;
use forge_foundation::PhaseType;

use crate::choice_space;
use crate::combat_choice_space;
use crate::gui_repro;
use crate::java_random::JavaRandom;
use crate::parity_card_map::ParityCardMap;

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_DIM_GRAY: &str = "\x1b[90m";
const ANSI_YELLOW: &str = "\x1b[33m";
const PREFER_ACTION_WEIGHT: usize = 3;

/// A hybrid deterministic agent: RNG for core decisions, fixed values for the rest.
pub struct DeterministicAgent {
    /// The player this agent controls.
    player_id: PlayerId,
    /// Log of notification messages (for debugging).
    pub log: Vec<String>,
    /// If true, print decisions to stderr.
    pub verbose: bool,
    /// Cached game state reference for name lookups.
    last_game_snapshot: Option<GameSnapshot>,
    /// Shared RNG for agent decisions — mirrors Java's `agentRng`.
    rng: Rc<RefCell<JavaRandom>>,
    /// Shared game RNG — mirrors Java's `MyRandom` (used for game effects like
    /// `Aggregates.random()` in random discard). This is the same instance used
    /// for deck shuffling, so its post-shuffle state matches Java's MyRandom.
    game_rng: Rc<RefCell<JavaRandom>>,
    /// If true, bias random main-phase choices toward taking an action over pass.
    prefer_actions: bool,
    /// Native Rust CardId -> shared parity id mapping.
    parity_map: Arc<ParityCardMap>,
}

/// Minimal cached state for looking up card names and types from IDs.
struct GameSnapshot {
    game: GameState,
    card_names: Vec<(CardId, String)>,
    card_is_land: Vec<(CardId, bool)>,
    ability_is_mana: Vec<((CardId, usize), bool)>,
}

#[derive(Clone, Copy)]
enum ActionChoice {
    Card(PlayOption),
    Ability(CardId, usize),
}

impl DeterministicAgent {
    pub fn new(
        player_id: PlayerId,
        verbose: bool,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        prefer_actions: bool,
        parity_map: Arc<ParityCardMap>,
    ) -> Self {
        Self {
            player_id,
            log: Vec::new(),
            verbose,
            last_game_snapshot: None,
            rng,
            game_rng,
            prefer_actions,
            parity_map,
        }
    }

    /// Look up a card name from the cached snapshot.
    fn card_name(&self, id: CardId) -> String {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, name) in &snap.card_names {
                if *cid == id {
                    return name.clone();
                }
            }
        }
        format!("Card({})", id.0)
    }

    /// Check if a card is a land from the cached snapshot.
    fn is_land(&self, id: CardId) -> bool {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, land) in &snap.card_is_land {
                if *cid == id {
                    return *land;
                }
            }
        }
        false
    }

    fn is_mana_ability(&self, card_id: CardId, ability_idx: usize) -> bool {
        if let Some(ref snap) = self.last_game_snapshot {
            for ((cid, idx), is_mana) in &snap.ability_is_mana {
                if *cid == card_id && *idx == ability_idx {
                    return *is_mana;
                }
            }
        }
        false
    }

    fn ability_sort_text(&self, card_id: CardId, ability_idx: usize) -> String {
        if let Some(ref snap) = self.last_game_snapshot {
            let card = snap.game.card(card_id);
            if let Some(ab) = card
                .activated_abilities
                .iter()
                .find(|ab| ab.ability_index == ability_idx)
            {
                return ab.ability_text.clone();
            }
        }
        String::new()
    }

    fn play_option_label(&self, play: PlayOption) -> String {
        if self.is_land(play.card_id) {
            return format!("LAND:{}", self.card_name(play.card_id));
        }
        let fb_tag = match play.mode {
            PlayCardMode::Alternative(AlternativeCost::Flashback) => "[FB]",
            _ => "",
        };
        format!("SPELL:{}{}", self.card_name(play.card_id), fb_tag)
    }

    fn play_option_sort_text(play: PlayOption) -> &'static str {
        match play.mode {
            PlayCardMode::Normal => "0",
            PlayCardMode::Alternative(AlternativeCost::Flashback) => {
                "Flashback"
            }
            PlayCardMode::Alternative(AlternativeCost::Spectacle) => {
                "Spectacle"
            }
            PlayCardMode::Alternative(AlternativeCost::Evoke) => "Evoke",
            PlayCardMode::Alternative(AlternativeCost::Dash) => "Dash",
            PlayCardMode::Alternative(AlternativeCost::Blitz) => "Blitz",
            PlayCardMode::Alternative(AlternativeCost::Escape) => "Escape",
            PlayCardMode::Alternative(AlternativeCost::Overload) => {
                "Overload"
            }
            PlayCardMode::Alternative(AlternativeCost::Madness) => "Madness",
            PlayCardMode::Alternative(AlternativeCost::Foretell) => {
                "Foretell"
            }
            PlayCardMode::Alternative(AlternativeCost::Emerge) => "Emerge",
            PlayCardMode::Alternative(AlternativeCost::Suspend) => "Suspend",
            PlayCardMode::GainLifeAlt => "GainLifeAlt",
            PlayCardMode::ForetellExile => "ForetellExile",
        }
    }

    fn action_sort_key(&self, choice: &ActionChoice) -> String {
        match *choice {
            ActionChoice::Card(play) => {
                let label = self.play_option_label(play);
                format!(
                    "{}|0|{}|{}|",
                    label,
                    self.parity_map.id(play.card_id),
                    Self::play_option_sort_text(play),
                )
            }
            ActionChoice::Ability(card_id, ability_idx) => format!(
                "AB:{}|1|{}|{:05}|{}",
                self.card_name(card_id),
                self.parity_map.id(card_id),
                ability_idx,
                self.ability_sort_text(card_id, ability_idx),
            ),
        }
    }

    fn legal_attackers_for_blocker(&self, blocker: CardId, attackers: &[CardId]) -> Vec<CardId> {
        let Some(ref snap) = self.last_game_snapshot else {
            return attackers.to_vec();
        };
        attackers
            .iter()
            .copied()
            .filter(|&attacker| {
                forge_engine_core::combat::can_creature_block(&snap.game, blocker, attacker)
            })
            .collect()
    }

    /// Pick a random index in [0, len) from the shared RNG.
    fn pick(&self, len: usize) -> usize {
        choice_space::pick_index(len, &mut self.rng.borrow_mut())
    }

    fn log_decision(&self, msg: &str) {
        if self.verbose {
            let styled = if msg.starts_with("Priority: PASS")
                || msg.contains("PASS (nothing playable)")
                || msg.contains("PASS (random)")
                || msg.contains("PASS (random weighted)")
                || msg.contains("PASS (non-sorcery-speed priority)")
            {
                format!("{ANSI_DIM_GRAY}{msg}{ANSI_RESET}")
            } else if msg.contains("Main phase: PLAY ")
                || msg.contains("Main phase: ACTIVATE ")
                || msg.starts_with("Attackers:")
                || msg.starts_with("Blockers:")
            {
                format!("{ANSI_YELLOW}{msg}{ANSI_RESET}")
            } else {
                msg.to_string()
            };
            eprintln!("[parity-agent p{}] {}", self.player_id.0, styled);
        }
    }
}

impl PlayerAgent for DeterministicAgent {
    fn snapshot_state(&mut self, game: &GameState, _mana_pools: &[ManaPool]) {
        // Cache card names, land status, and turn info for deterministic ordering
        let card_names: Vec<(CardId, String)> = game
            .cards
            .iter()
            .map(|c| (c.id, c.card_name.clone()))
            .collect();
        let card_is_land: Vec<(CardId, bool)> =
            game.cards.iter().map(|c| (c.id, c.is_land())).collect();
        let ability_is_mana: Vec<((CardId, usize), bool)> = game
            .cards
            .iter()
            .flat_map(|c| {
                c.activated_abilities
                    .iter()
                    .map(move |ab| ((c.id, ab.ability_index), ab.is_mana_ability))
            })
            .collect();
        self.last_game_snapshot = Some(GameSnapshot {
            game: game.clone(),
            card_names,
            card_is_land,
            ability_is_mana,
        });
    }

    fn mulligan_decision(
        &mut self,
        _player: PlayerId,
        _hand: &[CardId],
        _mulligan_count: u32,
    ) -> bool {
        self.log_decision("Mulligan: KEEP");
        true
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[PlayOption],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        if playable.is_empty() && activatable.is_empty() {
            self.log_decision("Main phase: PASS (nothing playable)");
            return MainPhaseAction::Pass;
        }

        // Match Java harness ActionSpace: omit explicit mana abilities from the
        // deterministic main action space.
        let filtered_activatable: Vec<(CardId, usize)> = activatable
            .iter()
            .copied()
            .filter(|(card_id, ability_idx)| !self.is_mana_ability(*card_id, *ability_idx))
            .collect();
        let choices: Vec<ActionChoice> = playable
            .iter()
            .copied()
            .into_iter()
            .map(ActionChoice::Card)
            .chain(
                filtered_activatable
                    .iter()
                    .copied()
                    .map(|(card_id, idx)| ActionChoice::Ability(card_id, idx)),
            )
            .collect();
        let choices =
            choice_space::sort_native(&choices, |a, b| self.action_sort_key(a).cmp(&self.action_sort_key(b)));
        if choices.is_empty() {
            self.log_decision("Main phase: PASS (no non-mana actions)");
            return MainPhaseAction::Pass;
        }
        // Pick randomly:
        // - default: each action + pass are equally likely
        // - prefer-actions: each action has weight PREFER_ACTION_WEIGHT, pass has weight 1
        let chosen_idx = if self.prefer_actions {
            let idx = choice_space::pick_weighted_index_with_pass(
                choices.len(),
                PREFER_ACTION_WEIGHT,
                &mut self.rng.borrow_mut(),
            );
            if idx >= choices.len() {
                self.log_decision("Main phase: PASS (random weighted)");
                return MainPhaseAction::Pass;
            }
            idx
        } else {
            let idx = choice_space::pick_index_with_pass(choices.len(), &mut self.rng.borrow_mut());
            if self.verbose {
                let opts: Vec<String> = choices
                    .iter()
                    .map(|c| match *c {
                        ActionChoice::Card(cid) => self.play_option_label(cid),
                        ActionChoice::Ability(cid, _) => format!("AB:{}", self.card_name(cid)),
                    })
                    .collect();
                eprintln!(
                    "[det-rust p{}] options={:?} idx={}/{}",
                    self.player_id.0,
                    opts,
                    idx,
                    choices.len()
                );
            }
            if idx >= choices.len() {
                self.log_decision("Main phase: PASS (random)");
                return MainPhaseAction::Pass;
            }
            idx
        };

        match choices[chosen_idx] {
            ActionChoice::Card(chosen) => {
                let name = self.card_name(chosen.card_id);
                self.log_decision(&format!(
                    "Main phase: PLAY {} (random idx={})",
                    name, chosen_idx
                ));
                MainPhaseAction::Play(chosen)
            }
            ActionChoice::Ability(card_id, ability_idx) => {
                let name = self.card_name(card_id);
                self.log_decision(&format!(
                    "Main phase: ACTIVATE {} [ab#{}] (random idx={})",
                    name, ability_idx, chosen_idx
                ));
                MainPhaseAction::ActivateAbility(card_id, ability_idx)
            }
        }
    }

    fn choose_attackers(
        &mut self,
        _player: PlayerId,
        available: &[CardId],
        possible_defenders: &[DefenderId],
    ) -> Vec<(CardId, DefenderId)> {
        let mut attackers = Vec::new();
        if !possible_defenders.is_empty() {
            let sorted_available = choice_space::sort_native(available, |a, b| {
                let an = self.card_name(*a);
                let bn = self.card_name(*b);
                an.cmp(&bn)
                    .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
            });
            for &id in &sorted_available {
                let roll = choice_space::pick_index(2, &mut self.rng.borrow_mut());
                if self.verbose {
                    eprintln!(
                        "[parity-agent p{}] atk roll {} -> {}",
                        self.player_id.0,
                        self.card_name(id),
                        roll
                    );
                }
                if roll == 1 {
                    let def_idx = choice_space::pick_index(
                        possible_defenders.len(),
                        &mut self.rng.borrow_mut(),
                    );
                    if self.verbose {
                        eprintln!(
                            "[parity-agent p{}] atk defender {} idx={}/{}",
                            self.player_id.0,
                            self.card_name(id),
                            def_idx,
                            possible_defenders.len()
                        );
                    }
                    attackers.push((id, possible_defenders[def_idx]));
                }
            }
        }
        if self.verbose && !attackers.is_empty() {
            let names: Vec<String> = attackers
                .iter()
                .map(|&(id, _)| self.card_name(id))
                .collect();
            self.log_decision(&format!("Attackers: {}", names.join(", ")));
        } else if self.verbose {
            self.log_decision("Attackers: NONE (random)");
        }
        attackers
    }

    fn exert_attackers(&mut self, _player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        if attackers.is_empty() {
            return vec![];
        }
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &attacker in attackers {
            if gui_repro::pick_bool(&mut rng) {
                out.push(attacker);
            }
        }
        out
    }

    fn enlist_attackers(&mut self, _player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        if attackers.is_empty() {
            return vec![];
        }
        choice_space::pick_one(attackers, &mut self.rng.borrow_mut())
            .into_iter()
            .collect()
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        let sorted_attackers = choice_space::sort_native(attackers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let sorted_blockers = choice_space::sort_native(available_blockers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });

        let mut pairs = Vec::new();
        for &blocker in &sorted_blockers {
            let legal_attackers = self.legal_attackers_for_blocker(blocker, &sorted_attackers);
            let choice = choice_space::pick_index_with_pass(
                legal_attackers.len(),
                &mut self.rng.borrow_mut(),
            );
            if choice > 0 && choice <= legal_attackers.len() {
                pairs.push((blocker, legal_attackers[choice - 1]));
            }
        }
        if pairs.is_empty() {
            self.log_decision("Blockers: NONE");
            return pairs;
        }
        if self.verbose && !pairs.is_empty() {
            let desc: Vec<String> = pairs
                .iter()
                .map(|(b, a)| format!("{} → {}", self.card_name(*b), self.card_name(*a)))
                .collect();
            self.log_decision(&format!("Blockers: {}", desc.join(", ")));
        } else if self.verbose {
            self.log_decision("Blockers: NONE (random)");
        }
        pairs
    }

    fn choose_blocker_for(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        blocker: CardId,
    ) -> Option<CardId> {
        let sorted_attackers = choice_space::sort_native(attackers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let legal_attackers = self.legal_attackers_for_blocker(blocker, &sorted_attackers);
        if legal_attackers.is_empty() {
            // Java DeterministicController always rolls `nextInt(options.size() + 1)`.
            // When options is empty, that's `nextInt(1)` (consumes RNG, always 0).
            let _ = choice_space::pick_index_with_pass(0, &mut self.rng.borrow_mut());
            if self.verbose {
                self.log_decision("Blockers: NONE");
            }
            return None;
        }
        let attacker = combat_choice_space::pick_single_blocker_target(
            &legal_attackers,
            &mut self.rng.borrow_mut(),
        );
        if attacker.is_none() {
            if self.verbose {
                self.log_decision("Blockers: NONE (random)");
            }
            return None;
        }
        let attacker = attacker.unwrap();
        if self.verbose {
            self.log_decision(&format!(
                "Blockers: {} → {}",
                self.card_name(blocker),
                self.card_name(attacker)
            ));
        }
        Some(attacker)
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        if valid.is_empty() {
            return None;
        }
        let target = choice_space::pick_one(valid, &mut self.rng.borrow_mut())?;
        self.log_decision(&format!("Target player: P{}", target.0));
        Some(target)
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let target = choice_space::pick_one(valid, &mut self.rng.borrow_mut())?;
        self.log_decision(&format!("Target card: {}", self.card_name(target)));
        Some(target)
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        _zone: forge_foundation::ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        self.choose_target_card(player, valid)
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        // Build unified list in engine-provided order.
        let total = valid_players.len() + valid_cards.len();

        if total == 0 {
            self.log_decision("Target any: NONE");
            return TargetChoice::None;
        }

        let idx = self.pick(total);
        if idx < valid_players.len() {
            let pid = valid_players[idx];
            self.log_decision(&format!("Target any: Player P{}", pid.0));
            TargetChoice::Player(pid)
        } else {
            let card_idx = idx - valid_players.len();
            let cid = valid_cards[card_idx];
            self.log_decision(&format!("Target any: Card {}", self.card_name(cid)));
            TargetChoice::Card(cid)
        }
    }

    // ── Trigger confirmation — mirrors Java AI brain logic (no RNG consumed) ──

    fn choose_optional_trigger(
        &mut self,
        _player: PlayerId,
        description: &str,
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        self.log_decision(&format!(
            "Optional trigger {} (api={:?}): {} [{}]",
            if accept { "ACCEPTED" } else { "DECLINED" },
            api,
            description,
            card_name.unwrap_or("?")
        ));
        accept
    }

    fn confirm_action(
        &mut self,
        _player: PlayerId,
        mode: Option<&str>,
        message: &str,
        _options: &[String],
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        self.log_decision(&format!(
            "Confirm action {} (mode={:?}, api={:?}): {} [{}]",
            if accept { "ACCEPTED" } else { "DECLINED" },
            mode,
            api,
            message,
            card_name.unwrap_or("?")
        ));
        accept
    }

    fn confirm_payment(
        &mut self,
        _player: PlayerId,
        cost_kind: &str,
        message: &str,
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        self.log_decision(&format!(
            "Confirm payment {} (cost_kind={}, api={:?}): {} [{}]",
            if accept { "ACCEPTED" } else { "DECLINED" },
            cost_kind,
            api,
            message,
            card_name.unwrap_or("?")
        ));
        accept
    }

    fn choose_binary(
        &mut self,
        _player: PlayerId,
        question: &str,
        kind: BinaryChoiceKind,
        _default_choice: Option<bool>,
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        let chosen_left = choice_space::pick_bool(&mut self.rng.borrow_mut());
        self.log_decision(&format!(
            "Choose binary {} (kind={}, api={:?}): {} [{}]",
            if chosen_left { "LEFT" } else { "RIGHT" },
            kind.as_str(),
            api,
            question,
            card_name.unwrap_or("?")
        ));
        chosen_left
    }

    // ── Fixed overrides that sort alphabetically (matching Java) but use no RNG ──

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        choice_space::pick_one(valid, &mut self.rng.borrow_mut())
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        if hand.is_empty() || num == 0 {
            return vec![];
        }
        gui_repro::pick_many_unique(hand, num, num, &mut self.rng.borrow_mut())
    }

    fn choose_random_discard(
        &mut self,
        _player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        if hand.is_empty() || num == 0 {
            return vec![];
        }
        // Reservoir sampling with the game RNG, mirroring Java's Aggregates.random()
        // which uses MyRandom.getRandom().nextInt(i) for reservoir replacement.
        // We use game_rng (not agent rng) to match Java's architecture where
        // Aggregates.random() uses MyRandom (the game-level RNG) rather than
        // the agent's decision RNG.
        // IMPORTANT: Do NOT sort — Java iterates cards in zone order (the order
        // they were added to hand), not alphabetically. Sorting would change the
        // reservoir sampling input sequence and produce different results.
        let count = num.min(hand.len());
        let mut rng = self.game_rng.borrow_mut();
        let mut result: Vec<CardId> = hand[..count].to_vec();
        for i in count..hand.len() {
            let j = choice_space::pick_index(i + 1, &mut rng);
            if j < count {
                result[j] = hand[i];
            }
        }
        result
    }

    fn choose_dig(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        _optional: bool,
    ) -> Vec<CardId> {
        if valid.is_empty() || max == 0 {
            return vec![];
        }
        gui_repro::pick_many_unique(valid, 0, max, &mut self.rng.borrow_mut())
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        // TODO: engine does not currently expose a typed choice list here.
        None
    }

    fn choose_color(&mut self, _player: PlayerId, valid_colors: &[String]) -> Option<String> {
        gui_repro::choose_color(valid_colors, &mut self.rng.borrow_mut())
    }

    fn choose_type(
        &mut self,
        _player: PlayerId,
        _type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
        gui_repro::choose_type(valid_types, &mut self.rng.borrow_mut())
    }

    fn choose_card_name(&mut self, _player: PlayerId, valid_names: &[String]) -> Option<String> {
        gui_repro::choose_card_name(valid_names, &mut self.rng.borrow_mut())
    }

    fn choose_number(&mut self, _player: PlayerId, min: i32, max: i32) -> Option<i32> {
        Some(gui_repro::choose_number(
            min,
            max,
            &mut self.rng.borrow_mut(),
        ))
    }

    fn choose_x_value(&mut self, _player: PlayerId, max_x: u32, _card_name: Option<&str>) -> u32 {
        max_x
    }

    fn pay_x_cost_in_mana(&self) -> bool {
        false
    }

    fn choose_cards_for_effect(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        if valid.is_empty() {
            return vec![];
        }
        gui_repro::pick_many_unique(valid, min, max, &mut self.rng.borrow_mut())
    }

    fn choose_single_card_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        _select_prompt: &str,
        _is_optional: bool,
    ) -> Option<CardId> {
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        self.choose_cards_for_effect(player, &sorted, 1, 1)
            .into_iter()
            .next()
    }

    fn choose_cards_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
        _select_prompt: &str,
    ) -> Vec<CardId> {
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        self.choose_cards_for_effect(player, &sorted, min, max)
    }

    fn choose_mode(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        _card_name: Option<&str>,
    ) -> Vec<usize> {
        if descriptions.is_empty() {
            return vec![];
        }
        let mut rng = self.rng.borrow_mut();
        let count = gui_repro::pick_count(min, max, descriptions.len(), &mut rng);
        let mut pool: Vec<usize> = (0..descriptions.len()).collect();
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            if pool.is_empty() {
                break;
            }
            let idx = choice_space::pick_index(pool.len(), &mut rng);
            out.push(pool.remove(idx));
        }
        out
    }

    fn choose_scry(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &cid in cards {
            if gui_repro::pick_bool(&mut rng) {
                out.push(cid);
            }
        }
        out
    }

    fn choose_surveil(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &cid in cards {
            if gui_repro::pick_bool(&mut rng) {
                out.push(cid);
            }
        }
        out
    }

    fn choose_reorder_library(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        gui_repro::shuffle_copy(cards, &mut self.rng.borrow_mut())
    }

    fn notify(&mut self, message: &str) {
        if self.log.len() >= 500 {
            self.log.remove(0);
        }
        self.log.push(message.to_string());
        if self.verbose {
            let styled = if message.starts_with("Played land:")
                || message.starts_with("Cast:")
                || message.starts_with("Activated ability:")
                || message.starts_with("Trigger fired:")
                || message.starts_with("Effect resolved:")
            {
                format!("{ANSI_YELLOW}{message}{ANSI_RESET}")
            } else {
                message.to_string()
            };
            eprintln!("[parity-agent p{}] notify: {}", self.player_id.0, styled);
        }
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        if self.verbose {
            eprintln!(
                "[parity-agent p{}] {}=== Turn {} (P{} active) ==={}",
                self.player_id.0, ANSI_DIM_GRAY, turn_number, active_player.0, ANSI_RESET
            );
        }
    }

    fn notify_phase_changed(&mut self, phase: PhaseType) {
        if self.verbose {
            eprintln!(
                "[parity-agent p{}] {}--- Phase: {:?} ---{}",
                self.player_id.0, ANSI_DIM_GRAY, phase, ANSI_RESET
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rng(seed: i64) -> Rc<RefCell<JavaRandom>> {
        Rc::new(RefCell::new(JavaRandom::new(seed)))
    }

    #[test]
    fn always_keeps_hand() {
        let mut agent = DeterministicAgent::new(
            PlayerId(0),
            false,
            make_rng(42),
            make_rng(42),
            false,
            Arc::new(ParityCardMap::default()),
        );
        assert!(agent.mulligan_decision(PlayerId(0), &[], 0));
    }

    #[test]
    fn random_target_player() {
        let rng = make_rng(42);
        let mut agent = DeterministicAgent::new(
            PlayerId(0),
            false,
            rng,
            make_rng(42),
            false,
            Arc::new(ParityCardMap::default()),
        );
        // With two valid targets, should randomly pick one
        let target = agent.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);
        assert!(target.is_some());
    }

    #[test]
    fn deterministic_across_runs() {
        // Same seed → same decisions
        let rng1 = make_rng(42);
        let mut agent1 = DeterministicAgent::new(
            PlayerId(0),
            false,
            rng1,
            make_rng(42),
            false,
            Arc::new(ParityCardMap::default()),
        );
        let t1 = agent1.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);

        let rng2 = make_rng(42);
        let mut agent2 = DeterministicAgent::new(
            PlayerId(0),
            false,
            rng2,
            make_rng(42),
            false,
            Arc::new(ParityCardMap::default()),
        );
        let t2 = agent2.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);

        assert_eq!(t1, t2);
    }
}
