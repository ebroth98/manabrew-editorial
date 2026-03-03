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

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_foundation::PhaseType;

use crate::java_random::JavaRandom;

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
    /// Current phase — used to only play spells during main phases.
    current_phase: Option<PhaseType>,
}

/// Minimal cached state for looking up card names and types from IDs.
struct GameSnapshot {
    card_names: Vec<(CardId, String)>,
    card_is_land: Vec<(CardId, bool)>,
    ability_sort_keys: Vec<(CardId, usize, String, String)>,
    active_player: PlayerId,
    phase: PhaseType,
    stack_empty: bool,
}

impl DeterministicAgent {
    pub fn new(
        player_id: PlayerId,
        verbose: bool,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        prefer_actions: bool,
    ) -> Self {
        Self {
            player_id,
            log: Vec::new(),
            verbose,
            last_game_snapshot: None,
            rng,
            game_rng,
            prefer_actions,
            current_phase: None,
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

    /// Sort card IDs alphabetically by their name.
    fn sort_by_name(&self, ids: &[CardId]) -> Vec<CardId> {
        let mut sorted: Vec<(CardId, String)> =
            ids.iter().map(|&id| (id, self.card_name(id))).collect();
        sorted.sort_by(|a, b| a.1.cmp(&b.1));
        sorted.into_iter().map(|(id, _)| id).collect()
    }

    /// Return Java-like sort key for activated abilities:
    /// host card name, then API, then original description text.
    fn ability_sort_key(&self, card_id: CardId, ability_idx: usize) -> (String, String) {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, idx, api, desc) in &snap.ability_sort_keys {
                if *cid == card_id && *idx == ability_idx {
                    return (api.clone(), desc.clone());
                }
            }
        }
        (String::new(), String::new())
    }

    /// Pick a random index in [0, len) from the shared RNG.
    fn pick(&self, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        // Important for parity: Java's Random.nextInt(1) still advances RNG state.
        // So we must consume RNG even when len == 1.
        self.rng.borrow_mut().next_int(len as i32) as usize
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
        let mut ability_sort_keys: Vec<(CardId, usize, String, String)> = Vec::new();
        for c in &game.cards {
            for ab in &c.activated_abilities {
                let api = ab.params.get("AB").cloned().unwrap_or_default();
                let desc = ab.params.get("SpellDescription").cloned().unwrap_or_default();
                ability_sort_keys.push((c.id, ab.ability_index, api, desc));
            }
        }
        self.last_game_snapshot = Some(GameSnapshot {
            card_names,
            card_is_land,
            ability_sort_keys,
            active_player: game.active_player(),
            phase: game.turn.phase,
            stack_empty: game.stack.is_empty(),
        });
    }

    fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId], _mulligan_count: u32) -> bool {
        self.log_decision("Mulligan: KEEP");
        true
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[CardId],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        // Only play spells during main phases — matches Java's AI behavior which
        // passes during non-main-phase priority windows (upkeep, combat, etc.).
        let is_main_phase = matches!(
            self.current_phase,
            Some(PhaseType::Main1) | Some(PhaseType::Main2)
        );
        if !is_main_phase {
            self.log_decision("Priority: PASS (non-main phase)");
            return MainPhaseAction::Pass;
        }
        if playable.is_empty() && activatable.is_empty() {
            self.log_decision("Main phase: PASS (nothing playable)");
            return MainPhaseAction::Pass;
        }

        // Only make RNG-based play decisions during the active player's
        // sorcery-speed window (main phase, our turn, stack empty).
        // This matches Java, where chooseSpellAbilityToPlay() is only called
        // for the active player during the main phase. Priority responses
        // (instants during opponent's turn or combat) go through the AI
        // framework in Java and never touch our shared RNG.
        if let Some(ref snap) = self.last_game_snapshot {
            let is_sorcery_speed = snap.active_player == self.player_id
                && matches!(snap.phase, PhaseType::Main1 | PhaseType::Main2)
                && snap.stack_empty;
            if !is_sorcery_speed {
                self.log_decision("Main phase: PASS (non-sorcery-speed priority)");
                return MainPhaseAction::Pass;
            }
        }

        // Partition into lands and spells, sort each alphabetically.
        // Lands come first in the combined list.
        let lands: Vec<CardId> = playable
            .iter()
            .copied()
            .filter(|&id| self.is_land(id))
            .collect();
        let spells: Vec<CardId> = playable
            .iter()
            .copied()
            .filter(|&id| !self.is_land(id))
            .collect();

        let sorted_lands = self.sort_by_name(&lands);
        let sorted_spells = self.sort_by_name(&spells);

        let cards: Vec<CardId> = sorted_lands
            .into_iter()
            .chain(sorted_spells)
            .collect();
        let mut abilities: Vec<(CardId, usize)> = activatable.to_vec();
        abilities.sort_by(|(card_a, idx_a), (card_b, idx_b)| {
            let name_a = self.card_name(*card_a);
            let name_b = self.card_name(*card_b);
            let key_a = self.ability_sort_key(*card_a, *idx_a);
            let key_b = self.ability_sort_key(*card_b, *idx_b);
            name_a
                .cmp(&name_b)
                .then(key_a.0.cmp(&key_b.0))
                .then(key_a.1.cmp(&key_b.1))
                .then(idx_a.cmp(idx_b))
        });

        enum ActionChoice {
            Card(CardId),
            Ability(CardId, usize),
        }
        let choices: Vec<ActionChoice> = cards
            .into_iter()
            .map(ActionChoice::Card)
            .chain(
                abilities
                    .into_iter()
                    .map(|(card_id, idx)| ActionChoice::Ability(card_id, idx)),
            )
            .collect();
        // Pick randomly:
        // - default: each action + pass are equally likely
        // - prefer-actions: each action has weight PREFER_ACTION_WEIGHT, pass has weight 1
        let chosen_idx = if self.prefer_actions {
            let total_weight = choices.len() * PREFER_ACTION_WEIGHT + 1;
            let roll = self.rng.borrow_mut().next_int(total_weight as i32) as usize;
            if roll >= choices.len() * PREFER_ACTION_WEIGHT {
                self.log_decision("Main phase: PASS (random weighted)");
                return MainPhaseAction::Pass;
            }
            roll / PREFER_ACTION_WEIGHT
        } else {
            let idx = self.rng.borrow_mut().next_int((choices.len() + 1) as i32) as usize;
            if self.verbose {
                let opts: Vec<String> = choices
                    .iter()
                    .map(|c| match *c {
                        ActionChoice::Card(cid) => {
                            if self.is_land(cid) {
                                format!("LAND:{}", self.card_name(cid))
                            } else {
                                format!("SPELL:{}", self.card_name(cid))
                            }
                        }
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
                let name = self.card_name(chosen);
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

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        // Sort eligible creatures alphabetically, then per-creature coin flip
        let sorted = self.sort_by_name(available);
        let mut attackers = Vec::new();
        for &id in &sorted {
            if self.rng.borrow_mut().next_int(2) == 1 {
                attackers.push(id);
            }
        }
        if self.verbose && !attackers.is_empty() {
            let names: Vec<String> = attackers.iter().map(|&id| self.card_name(id)).collect();
            self.log_decision(&format!("Attackers: {}", names.join(", ")));
        } else if self.verbose {
            self.log_decision("Attackers: NONE (random)");
        }
        attackers
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        if attackers.is_empty() || available_blockers.is_empty() {
            self.log_decision("Blockers: NONE");
            return Vec::new();
        }

        let sorted_blockers = self.sort_by_name(available_blockers);
        let sorted_attackers = self.sort_by_name(attackers);

        // For each blocker: 0 = don't block, 1..=count = block that attacker
        let mut pairs = Vec::new();
        for &blocker in &sorted_blockers {
            let choice = self.rng.borrow_mut().next_int((sorted_attackers.len() + 1) as i32) as usize;
            if choice > 0 && choice <= sorted_attackers.len() {
                pairs.push((blocker, sorted_attackers[choice - 1]));
            }
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

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        if valid.is_empty() {
            return None;
        }
        // Sort by player index for determinism
        let mut sorted = valid.to_vec();
        sorted.sort_by_key(|p| p.0);
        let idx = self.pick(sorted.len());
        let target = sorted[idx];
        self.log_decision(&format!("Target player: P{}", target.0));
        Some(target)
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let sorted = self.sort_by_name(valid);
        let idx = self.pick(sorted.len());
        let target = sorted[idx];
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
        // Build unified list: players first (sorted by index), then cards (sorted by name).
        // This matches Java's chooseSingleEntityForEffect sorting.
        let mut sorted_players = valid_players.to_vec();
        sorted_players.sort_by_key(|p| p.0);

        let sorted_cards = self.sort_by_name(valid_cards);
        let total = sorted_players.len() + sorted_cards.len();

        if total == 0 {
            self.log_decision("Target any: NONE");
            return TargetChoice::None;
        }

        let idx = self.pick(total);
        if idx < sorted_players.len() {
            let pid = sorted_players[idx];
            self.log_decision(&format!("Target any: Player P{}", pid.0));
            TargetChoice::Player(pid)
        } else {
            let card_idx = idx - sorted_players.len();
            let cid = sorted_cards[card_idx];
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
        // Mirror Java's AI brain behavior for optional triggers:
        // - PumpAi.doTriggerNoCost() returns NO for optional non-targeted pump
        // - PumpAllAi inherits the same behavior
        // This matches DeterministicController delegating confirmTrigger to the
        // AI brain (PlayerControllerAi → AiController.doTrigger → SpellApiToAi).
        let decline = matches!(api, Some("Pump" | "PumpAll"));
        if decline {
            self.log_decision(&format!(
                "Optional trigger DECLINED (api={:?}): {} [{}]",
                api,
                description,
                card_name.unwrap_or("?")
            ));
            return false;
        }
        self.log_decision(&format!(
            "Optional trigger ACCEPTED (api={:?}): {} [{}]",
            api,
            description,
            card_name.unwrap_or("?")
        ));
        true
    }

    // ── Fixed overrides that sort alphabetically (matching Java) but use no RNG ──

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let sorted = self.sort_by_name(valid);
        Some(sorted[0])
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        let sorted = self.sort_by_name(hand);
        let count = num.min(sorted.len());
        sorted[..count].to_vec()
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
            let j = rng.next_int(i as i32 + 1) as usize;
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
        let sorted = self.sort_by_name(valid);
        let count = max.min(sorted.len());
        sorted[..count].to_vec()
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None // fixed: no RNG consumed
    }

    fn notify(&mut self, message: &str) {
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
        self.current_phase = Some(phase);
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
        let mut agent = DeterministicAgent::new(PlayerId(0), false, make_rng(42), make_rng(42), false);
        assert!(agent.mulligan_decision(PlayerId(0), &[], 0));
    }

    #[test]
    fn random_target_player() {
        let rng = make_rng(42);
        let mut agent = DeterministicAgent::new(PlayerId(0), false, rng, make_rng(42), false);
        // With two valid targets, should randomly pick one
        let target = agent.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);
        assert!(target.is_some());
    }

    #[test]
    fn deterministic_across_runs() {
        // Same seed → same decisions
        let rng1 = make_rng(42);
        let mut agent1 =
            DeterministicAgent::new(PlayerId(0), false, rng1, make_rng(42), false);
        let t1 = agent1.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);

        let rng2 = make_rng(42);
        let mut agent2 =
            DeterministicAgent::new(PlayerId(0), false, rng2, make_rng(42), false);
        let t2 = agent2.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);

        assert_eq!(t1, t2);
    }
}
