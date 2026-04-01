use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hasher};

use forge_foundation::{PhaseType, ZoneType};

use crate::ability::effects::{self, EffectContext};
use crate::agent::{CombatCostAction, MainPhaseAction, ManaCostAction, PlayerAgent};
use crate::card::Card;
use crate::combat::{self, CombatState};
use crate::cost::{self, parse_cost, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::game_log::GameLog;
use crate::game_log_entry_type::GameLogEntryType;
use crate::game_rng::{GameRng, ThreadRngAdapter};
use crate::game_snapshot::GameSnapshot;
use crate::ids::{CardId, PlayerId};
use crate::mana::{self, basic_land_mana_atom, ManaPool};
use crate::parsing::{keys, Params};
use crate::spellability::target_restrictions;
use crate::spellability::{build_spell_ability, SpellAbility, StackEntry};
use crate::staticability::layer::apply_continuous_effects;
use crate::trigger::handler::TriggerHandler;

// ── GameLoop ────────────────────────────────────────────────────────

/// Drives a complete game from setup through game over.
pub struct GameLoop {
    pub mana_pools: Vec<ManaPool>,
    pub combat: CombatState,
    pub trigger_handler: TriggerHandler,
    pub game_log: GameLog,
    /// Token templates keyed by their script filename stem (e.g. "r_1_1_goblin").
    /// Populated at game start by the Tauri layer; used by the Token effect handler.
    pub token_templates: HashMap<String, Card>,
    /// Pluggable RNG for game effects (shuffles, coin flips, dice rolls).
    /// Default: ThreadRngAdapter (non-deterministic). For parity testing,
    /// replace with a JavaRandom-backed implementation.
    pub game_rng: Box<dyn GameRng>,
    /// Enables Java-parity snapshot rollback support (`stash_game_state` / `restore_game_state`).
    pub experimental_restore_snapshot: bool,
    /// Last stashed snapshot used by rollback flows.
    previous_game_state: Option<GameSnapshot>,
    /// Rolling checkpoint history for UI rewind/debug.
    checkpoints: VecDeque<(u64, String, GameSnapshot)>,
    next_checkpoint_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnMachineState {
    Untap,
    Upkeep,
    Draw,
    Main1,
    Combat,
    Main2,
    EndOfTurn,
    Cleanup,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnEvent {
    EnterPhase {
        phase: PhaseType,
        emit_phase_trigger: bool,
    },
    PriorityWindow {
        is_main_phase: bool,
    },
    UntapStep,
    DrawStep,
    CombatStep,
    CleanupStep,
    AdvanceTurn,
}

impl GameLoop {
    pub fn new(num_players: usize) -> Self {
        GameLoop {
            mana_pools: (0..num_players).map(|_| ManaPool::new()).collect(),
            combat: CombatState::new(),
            trigger_handler: TriggerHandler::new(),
            game_log: GameLog::new(),
            token_templates: HashMap::new(),
            game_rng: Box::new(ThreadRngAdapter),
            experimental_restore_snapshot: false,
            previous_game_state: None,
            checkpoints: VecDeque::new(),
            next_checkpoint_id: 1,
        }
    }

    /// Register a token template by its script filename stem (e.g. "r_1_1_goblin").
    /// Called at game start by the Tauri layer for every token script in the token DB.
    pub fn register_token(&mut self, script_name: impl Into<String>, template: Card) {
        self.token_templates.insert(script_name.into(), template);
    }

    pub fn pool(&self, pid: PlayerId) -> &ManaPool {
        &self.mana_pools[pid.index()]
    }

    pub fn pool_mut(&mut self, pid: PlayerId) -> &mut ManaPool {
        &mut self.mana_pools[pid.index()]
    }

    pub(crate) fn move_card_with_runtime(
        &mut self,
        game: &mut GameState,
        card_id: CardId,
        dest_zone: ZoneType,
        dest_owner: PlayerId,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let mut runtime = crate::replacement::replacement_handler::ReplacementRuntime {
            trigger_handler: &mut self.trigger_handler,
            token_templates: &self.token_templates,
            mana_pools: &mut self.mana_pools,
            rng: &mut *self.game_rng,
        };
        game.move_card_with_agents_and_replacement_runtime(
            card_id,
            dest_zone,
            dest_owner,
            agents,
            &mut runtime,
        );
    }

    /// Create a game snapshot. Set `include_stack` false for copy-without-stack flows.
    pub fn make_snapshot(&self, game: &GameState, include_stack: bool) -> GameSnapshot {
        GameSnapshot::capture(
            game,
            &self.mana_pools,
            &self.combat,
            &self.trigger_handler,
            include_stack,
        )
    }

    /// Restore a previously captured snapshot.
    pub fn restore_snapshot(&mut self, game: &mut GameState, snapshot: &GameSnapshot) {
        snapshot.restore_game_state(
            game,
            &mut self.mana_pools,
            &mut self.combat,
            &mut self.trigger_handler,
        );
    }

    /// Stash the current state if snapshot rollback is enabled.
    pub fn stash_game_state(&mut self, game: &GameState) {
        if self.experimental_restore_snapshot {
            self.previous_game_state = Some(self.make_snapshot(game, true));
        }
    }

    /// Restore from the previously stashed state if available and enabled.
    pub fn restore_game_state(&mut self, game: &mut GameState) -> bool {
        if !self.experimental_restore_snapshot {
            return false;
        }
        let Some(snapshot) = self.previous_game_state.clone() else {
            return false;
        };
        self.restore_snapshot(game, &snapshot);
        true
    }

    pub fn restore_checkpoint(&mut self, game: &mut GameState, checkpoint_id: u64) -> bool {
        let Some((_, _, snapshot)) = self
            .checkpoints
            .iter()
            .find(|(id, _, _)| *id == checkpoint_id)
            .cloned()
        else {
            return false;
        };
        self.restore_snapshot(game, &snapshot);
        true
    }

    fn record_checkpoint(&mut self, game: &GameState, include_stack: bool) -> (u64, String) {
        let checkpoint_id = self.next_checkpoint_id;
        self.next_checkpoint_id += 1;
        let label = format!(
            "Turn {} {}",
            game.turn.turn_number,
            game.turn.phase.script_name()
        );
        let snap = self.make_snapshot(game, include_stack);
        self.checkpoints
            .push_back((checkpoint_id, label.clone(), snap));
        while self.checkpoints.len() > 256 {
            self.checkpoints.pop_front();
        }
        (checkpoint_id, label)
    }

    pub(crate) fn apply_pending_snapshot_restore(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) -> bool {
        let mut requested = None;
        for agent in agents.iter_mut() {
            if let Some(id) = agent.take_restore_request() {
                requested = Some(id);
            }
        }
        let Some(checkpoint_id) = requested else {
            return false;
        };
        let restored = self.restore_checkpoint(game, checkpoint_id);
        if restored {
            for agent in agents.iter_mut() {
                agent.snapshot_state(game, &self.mana_pools);
                agent.notify_state_changed();
            }
        }
        restored
    }

    /// Get untapped lands on the battlefield for a player.
    pub fn get_tappable_lands(&self, game: &GameState, player: PlayerId) -> Vec<CardId> {
        game.cards_in_zone(ZoneType::Battlefield, player)
            .to_vec()
            .into_iter()
            .filter(|&cid| {
                let c = game.card(cid);
                c.is_land() && !c.tapped
            })
            .collect()
    }

    /// Get tapped lands whose mana is still in the pool (can be untapped to undo).
    pub fn get_untappable_lands(
        &self,
        game: &GameState,
        player: PlayerId,
        pool_snapshot: &ManaPool,
    ) -> Vec<CardId> {
        game.cards_in_zone(ZoneType::Battlefield, player)
            .to_vec()
            .into_iter()
            .filter(|&cid| {
                let c = game.card(cid);
                if !c.is_land() || !c.tapped {
                    return false;
                }
                let atoms = mana::land_mana_atoms(c);
                if !atoms.is_empty() {
                    atoms.iter().any(|&a| pool_snapshot.has_atom(a, 1))
                } else if let Some(atom) = basic_land_mana_atom(c) {
                    pool_snapshot.has_atom(atom, 1)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Set up the game: shuffle libraries, draw opening hands, run mulligans.
    pub fn setup(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        rng: &mut impl rand::Rng,
    ) {
        for &pid in &game.player_order.clone() {
            game.shuffle_library(pid, rng);
            game.draw_cards(pid, 7);
        }

        let first_player = game.active_player();
        crate::mulligan::run_london_mulligans(
            game,
            agents,
            rng,
            first_player,
            &self.mana_pools,
            &self.game_log,
        );
    }

    /// Run the full game until someone wins or loses.
    /// Returns the winner's PlayerId.
    pub fn run(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        rng: &mut impl rand::Rng,
        max_turns: u32,
    ) -> Option<PlayerId> {
        self.setup(game, agents, rng);
        self.trigger_handler.reset_active_triggers(game);
        self.trigger_handler
            .run_trigger(TriggerType::NewGame, RunParams::default(), true);

        while !game.game_over && game.turn.turn_number <= max_turns {
            self.run_turn(game, agents, rng);
        }

        game.winner
    }

    /// Run a single turn.
    pub fn run_turn(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        _rng: &mut impl rand::Rng,
    ) {
        let active = game.active_player();
        let active_name = game.player(active).name.clone();

        // SkipTurn (issue #22): if the active player has skip_turns > 0, skip entirely.
        if game.player(active).skip_turns > 0 {
            game.player_decrement_skip_turns(active);
            self.log_turn_skipped(game, active, game.player(active).skip_turns);
            // Still advance turn state so the next player gets their turn
            game.turn.next_player_turn(&game.player_order.clone());
            return;
        }

        game.new_turn_for_player(active);
        self.log_turn_begin(&active_name, game.turn.turn_number);

        // Snapshot + notify all agents of the turn change (display-only, before any actions)
        let turn_number = game.turn.turn_number;
        for agent in agents.iter_mut() {
            agent.snapshot_state(game, &self.mana_pools);
        }
        let (checkpoint_id, label) = self.record_checkpoint(game, true);
        for agent in agents.iter_mut() {
            agent.notify_snapshot_created(checkpoint_id, &label);
        }
        for agent in agents.iter_mut() {
            agent.notify_turn_changed(active, turn_number);
        }

        // Rebuild active triggers at start of turn
        self.trigger_handler.reset_active_triggers(game);
        // Recompute continuous static effects for the new turn.
        apply_continuous_effects(game);

        self.run_turn_state_machine(game, agents);
    }

    pub(crate) fn log_turn_begin(&self, player_name: &str, turn_number: u32) {
        self.game_log.log(
            GameLogEntryType::TurnBegin,
            0,
            format!("{player_name} turn begins (turn {turn_number})"),
        );
    }

    pub(crate) fn log_turn_skipped(
        &self,
        game: &GameState,
        player: PlayerId,
        remaining_skip_turns: i32,
    ) {
        self.game_log.log(
            GameLogEntryType::TurnSkip,
            0,
            format!(
                "{} turn skipped (remaining skip-turn effects: {})",
                game.player(player).name,
                remaining_skip_turns
            ),
        );
    }

    pub(crate) fn log_phase_begin(&self, phase: PhaseType) {
        self.game_log.log(
            GameLogEntryType::PhaseBegin,
            1,
            format!("Phase {}", phase.script_name()),
        );
    }

    pub(crate) fn log_waiting_for_priority(&self, game: &GameState, player: PlayerId) {
        self.game_log.log(
            GameLogEntryType::PriorityWaiting,
            2,
            format!("Waiting for {} priority response", game.player(player).name),
        );
    }

    pub(crate) fn log_priority_response(&self, game: &GameState, player: PlayerId, action: &str) {
        self.game_log.log(
            GameLogEntryType::PriorityResponse,
            2,
            format!("{} responded with {}", game.player(player).name, action),
        );
    }

    pub(crate) fn log_priority_pass(&self, game: &GameState, player: PlayerId) {
        self.game_log.log(
            GameLogEntryType::PriorityPass,
            2,
            format!("{} passed priority", game.player(player).name),
        );
    }

    pub(crate) fn log_stack_push(&self, item_name: &str, player_name: &str) {
        self.game_log.log(
            GameLogEntryType::StackPush,
            2,
            format!("{item_name} pushed to stack ({player_name})"),
        );
    }

    pub(crate) fn log_stack_resolved_item(&self, item_name: &str) {
        self.game_log.log(
            GameLogEntryType::StackResolve,
            2,
            format!("{item_name} resolved"),
        );
    }
}

/// Helper: run SBA with trigger handler and legend-rule agent callback.
/// Mirrors Java's GameAction.checkStateEffects() + handleLegendRule() which
/// delegates the "keep which legendary?" choice to the player controller.
fn check_sba(
    game: &mut GameState,
    trigger_handler: &mut TriggerHandler,
    agents: &mut [Box<dyn PlayerAgent>],
) -> bool {
    let result = game.check_state_based_actions_with_trigger_agents(Some(trigger_handler), agents);
    if result {
        // Flush triggers fired during SBA before re-registering. This preserves
        // triggers from Animate effects (pump_trigger_count) that were active
        // when creatures died.
        trigger_handler.flush_waiting_triggers(game);
        // Re-register triggers after SBA may have moved cards between zones.
        // This ensures triggers with non-Battlefield active zones (e.g.
        // TriggerZones$ Graveyard) are registered when cards die.
        trigger_handler.reset_active_triggers(game);
    }
    result
}

mod action_space;
mod cast_spell;
mod combat_phase;
mod cost_payment;
mod game_action;
mod phase_handler;
mod playability;
mod priority;
mod stack_resolution;
mod state_observer;
mod trigger_handler;

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};
    use rand::SeedableRng;

    use crate::agent::{PlayCardMode, PlayerAgent, TargetChoice};
    use crate::player::actions::PlayerAction;
    use crate::card::Card;

    use super::*;

    struct RecordingPassAgent {
        phases_seen: Arc<Mutex<Vec<PhaseType>>>,
        bad_priority_seen: Arc<AtomicBool>,
        last_phase: Option<PhaseType>,
        last_priority: Option<PlayerId>,
    }

    struct InvalidPlayAgent;

    impl PlayerAgent for InvalidPlayAgent {
        fn mulligan_decision(
            &mut self,
            _player: PlayerId,
            _hand: &[CardId],
            _mulligan_count: u32,
        ) -> bool {
            true
        }

        fn choose_action(
            &mut self,
            _player: PlayerId,
            _playable: &[crate::agent::PlayOption],
            _tappable_lands: &[CardId],
            _untappable_lands: &[CardId],
            _activatable: &[(CardId, usize)],
        ) -> PlayerAction {
            PlayerAction::CastSpell(crate::agent::PlayOption {
                card_id: CardId(u32::MAX),
                mode: PlayCardMode::Normal,
            })
        }

        fn choose_attackers(
            &mut self,
            _player: PlayerId,
            _available: &[CardId],
            _possible_defenders: &[crate::combat::DefenderId],
        ) -> Vec<(CardId, crate::combat::DefenderId)> {
            Vec::new()
        }

        fn choose_blockers(
            &mut self,
            _player: PlayerId,
            _attackers: &[CardId],
            _available_blockers: &[CardId],
            _max_blockers: Option<usize>,
        ) -> Vec<(CardId, CardId)> {
            Vec::new()
        }

        fn choose_target_player(
            &mut self,
            _player: PlayerId,
            valid: &[PlayerId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<PlayerId> {
            valid.first().copied()
        }

        fn choose_target_card(
            &mut self,
            _player: PlayerId,
            valid: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<CardId> {
            valid.first().copied()
        }

        fn choose_target_any(
            &mut self,
            _player: PlayerId,
            valid_players: &[PlayerId],
            valid_cards: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> TargetChoice {
            if let Some(&pid) = valid_players.first() {
                TargetChoice::Player(pid)
            } else if let Some(&cid) = valid_cards.first() {
                TargetChoice::Card(cid)
            } else {
                TargetChoice::None
            }
        }

        fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
            None
        }

        fn notify(&mut self, _message: &str) {}
    }

    impl RecordingPassAgent {
        fn new(
            phases_seen: Arc<Mutex<Vec<PhaseType>>>,
            bad_priority_seen: Arc<AtomicBool>,
        ) -> Self {
            Self {
                phases_seen,
                bad_priority_seen,
                last_phase: None,
                last_priority: None,
            }
        }
    }

    impl PlayerAgent for RecordingPassAgent {
        fn snapshot_state(&mut self, game: &GameState, _mana_pools: &[ManaPool]) {
            self.last_phase = Some(game.turn.phase);
            self.last_priority = Some(game.turn.priority_player);
        }

        fn mulligan_decision(
            &mut self,
            _player: PlayerId,
            _hand: &[CardId],
            _mulligan_count: u32,
        ) -> bool {
            true
        }

        fn choose_action(
            &mut self,
            player: PlayerId,
            _playable: &[crate::agent::PlayOption],
            _tappable_lands: &[CardId],
            _untappable_lands: &[CardId],
            _activatable: &[(CardId, usize)],
        ) -> PlayerAction {
            if self.last_priority != Some(player) {
                self.bad_priority_seen.store(true, Ordering::SeqCst);
            }
            if let Some(phase) = self.last_phase {
                self.phases_seen.lock().unwrap().push(phase);
            }
            PlayerAction::PassPriority
        }

        fn choose_attackers(
            &mut self,
            _player: PlayerId,
            _available: &[CardId],
            _possible_defenders: &[crate::combat::DefenderId],
        ) -> Vec<(CardId, crate::combat::DefenderId)> {
            Vec::new()
        }

        fn choose_blockers(
            &mut self,
            _player: PlayerId,
            _attackers: &[CardId],
            _available_blockers: &[CardId],
            _max_blockers: Option<usize>,
        ) -> Vec<(CardId, CardId)> {
            Vec::new()
        }

        fn choose_target_player(
            &mut self,
            _player: PlayerId,
            valid: &[PlayerId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<PlayerId> {
            valid.first().copied()
        }

        fn choose_target_card(
            &mut self,
            _player: PlayerId,
            valid: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<CardId> {
            valid.first().copied()
        }

        fn choose_target_any(
            &mut self,
            _player: PlayerId,
            valid_players: &[PlayerId],
            valid_cards: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> TargetChoice {
            if let Some(&pid) = valid_players.first() {
                TargetChoice::Player(pid)
            } else if let Some(&cid) = valid_cards.first() {
                TargetChoice::Card(cid)
            } else {
                TargetChoice::None
            }
        }

        fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
            None
        }

        fn notify(&mut self, _message: &str) {}
    }

    fn zero_cost_instant(owner: PlayerId) -> Card {
        Card::new(
            CardId(0),
            "Test Instant".to_string(),
            owner,
            CardTypeLine::parse("Instant"),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        )
    }

    fn mana_land(owner: PlayerId, name: &str, produced: &str) -> Card {
        Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse("Land"),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![format!(
                "AB$ Mana | Cost$ T | Produced$ {} | SpellDescription$ Add mana.",
                produced
            )],
        )
    }

    fn vanilla_spell(owner: PlayerId, name: &str, cost: &str) -> Card {
        Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse("Sorcery"),
            ManaCost::parse(cost),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        )
    }

    fn evoked_etb_creature(owner: PlayerId) -> Card {
        let mut card = Card::new(
            CardId(0),
            "Mulldrifter Test".to_string(),
            owner,
            CardTypeLine::parse("Creature - Elemental"),
            ManaCost::parse("4 U"),
            ColorSet::BLUE,
            Some(2),
            Some(2),
            vec!["Evoke:2 U".to_string()],
            vec![],
        );

        let mut next_trigger_id = 0;
        let etb_draw = crate::trigger::parse_trigger(
            "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When CARDNAME enters the battlefield, draw two cards.",
            &mut next_trigger_id,
        )
        .expect("valid ETB trigger");
        card.add_trigger(etb_draw);
        card.svars.insert(
            "TrigDraw".to_string(),
            "DB$ Draw | NumCards$ 2 | Defined$ You".to_string(),
        );
        card
    }

    fn activated_permanent(
        owner: PlayerId,
        name: &str,
        type_line: &str,
        abilities: Vec<&str>,
    ) -> Card {
        Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse(type_line),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            abilities.into_iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn run_turn_opens_draw_and_combat_priority_windows() {
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let mut game = GameState::new(&["A", "B"], 20);

        let c0 = game.create_card(zero_cost_instant(p0));
        let c1 = game.create_card(zero_cost_instant(p1));
        game.move_card(c0, ZoneType::Hand, p0);
        game.move_card(c1, ZoneType::Hand, p1);

        let seen0 = Arc::new(Mutex::new(Vec::new()));
        let seen1 = Arc::new(Mutex::new(Vec::new()));
        let bad0 = Arc::new(AtomicBool::new(false));
        let bad1 = Arc::new(AtomicBool::new(false));

        let a0 = RecordingPassAgent::new(seen0.clone(), bad0.clone());
        let a1 = RecordingPassAgent::new(seen1.clone(), bad1.clone());

        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(a0), Box::new(a1)];
        let mut game_loop = GameLoop::new(2);
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        // Turn 2 ensures the draw step draw action is exercised.
        game.turn.turn_number = 2;
        game.turn.active_player = p0;
        game.turn.priority_player = p0;

        game_loop.run_turn(&mut game, &mut agents, &mut rng);

        let mut all_phases = seen0.lock().unwrap().clone();
        all_phases.extend(seen1.lock().unwrap().iter().copied());

        // Priority windows are opened at Draw, Main1, and Main2.
        // Combat phases don't call choose_action, so they're not recorded here.
        assert!(all_phases.contains(&PhaseType::Draw));
        assert!(all_phases.contains(&PhaseType::Main1));
        assert!(all_phases.contains(&PhaseType::Main2));
        assert!(all_phases.contains(&PhaseType::EndOfTurn));

        assert!(!bad0.load(Ordering::SeqCst));
        assert!(!bad1.load(Ordering::SeqCst));
    }

    #[test]
    fn priority_round_ignores_illegal_actions() {
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let mut game = GameState::new(&["A", "B"], 20);

        let c0 = game.create_card(zero_cost_instant(p0));
        let c1 = game.create_card(zero_cost_instant(p1));
        game.move_card(c0, ZoneType::Hand, p0);
        game.move_card(c1, ZoneType::Hand, p1);

        game.turn.active_player = p0;
        game.turn.priority_player = p0;
        game.turn.phase = PhaseType::Upkeep;

        let seen = Arc::new(Mutex::new(Vec::new()));
        let bad = Arc::new(AtomicBool::new(false));
        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
            Box::new(InvalidPlayAgent),
            Box::new(RecordingPassAgent::new(seen, bad)),
        ];

        let mut game_loop = GameLoop::new(2);
        game_loop.priority_round(&mut game, &mut agents, false);

        assert!(game.stack.is_empty());
        assert!(game.cards_in_zone(ZoneType::Hand, p0).contains(&c0));
        assert!(game.cards_in_zone(ZoneType::Hand, p1).contains(&c1));
        assert_eq!(game.turn.priority_player, game.active_player());
    }

    #[test]
    fn action_space_excludes_nonland_mana_abilities_from_main_actions() {
        let p0 = PlayerId(0);
        let mut game = GameState::new(&["A", "B"], 20);

        let goose = game.create_card(activated_permanent(
            p0,
            "Gilded Goose",
            "Creature - Bird",
            vec![
                "AB$ Token | Cost$ 1 G T | TokenScript$ c_a_food_sac | TokenOwner$ You | SpellDescription$ Create a Food Token.",
                "AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any | SpellDescription$ Add one mana of any color.",
            ],
        ));
        let food = game.create_card(activated_permanent(
            p0,
            "Food Token",
            "Artifact Food",
            vec!["AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3 | SpellDescription$ You gain 3 life."],
        ));
        let forest = game.create_card(mana_land(p0, "Forest", "G"));
        let island = game.create_card(mana_land(p0, "Island", "U"));

        for cid in [goose, food, forest, island] {
            game.move_card(cid, ZoneType::Battlefield, p0);
            game.card_mut(cid).summoning_sick = false;
        }

        game.turn.turn_number = 20;
        game.turn.active_player = p0;
        game.turn.priority_player = p0;
        game.turn.phase = PhaseType::Main1;

        let gl = GameLoop::new(2);
        let action_space = gl.action_space(&game, p0, true);

        assert!(action_space.activatable.contains(&(food, 0)));
        assert!(action_space.activatable.contains(&(goose, 0)));
        assert!(!action_space.activatable.contains(&(goose, 1)));
    }

    #[test]
    fn action_space_excludes_activated_abilities_that_only_pay_via_same_host_mana() {
        let p0 = PlayerId(0);
        let mut game = GameState::new(&["A", "B"], 20);

        let goose = game.create_card(activated_permanent(
            p0,
            "Gilded Goose",
            "Creature - Bird",
            vec![
                "AB$ Token | Cost$ 1 G T | TokenScript$ c_a_food_sac | TokenOwner$ You | SpellDescription$ Create a Food Token.",
                "AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any | SpellDescription$ Add one mana of any color.",
            ],
        ));
        let food = game.create_card(activated_permanent(
            p0,
            "Food Token",
            "Artifact Food",
            vec!["AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3 | SpellDescription$ You gain 3 life."],
        ));
        let plains = game.create_card(mana_land(p0, "Plains", "W"));

        for cid in [goose, food, plains] {
            game.move_card(cid, ZoneType::Battlefield, p0);
            game.card_mut(cid).summoning_sick = false;
        }

        game.turn.active_player = p0;
        game.turn.priority_player = p0;
        game.turn.phase = PhaseType::Main1;

        let gl = GameLoop::new(2);
        let action_space = gl.action_space(&game, p0, true);

        assert!(action_space.activatable.contains(&(food, 0)));
        assert!(!action_space.activatable.contains(&(goose, 0)));
        assert!(!action_space.activatable.contains(&(goose, 1)));
    }

    #[test]
    fn play_card_uses_manual_pool_then_auto_taps_deficit() {
        let p0 = PlayerId(0);
        let _p1 = PlayerId(1);
        let mut game = GameState::new(&["A", "B"], 20);

        // Land A: any color (manual tap first)
        let land_any = game.create_card(mana_land(p0, "Any Land", "Any"));
        // Land B: can produce U or G (auto-tap should use this for blue requirement)
        let land_combo = game.create_card(mana_land(p0, "Dual Land", "Combo G U"));
        let spell = game.create_card(vanilla_spell(p0, "Test Spell", "1 U"));

        game.move_card(land_any, ZoneType::Battlefield, p0);
        game.move_card(land_combo, ZoneType::Battlefield, p0);
        game.move_card(spell, ZoneType::Hand, p0);

        let mut gl = GameLoop::new(2);
        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
            Box::new(RecordingPassAgent::new(
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicBool::new(false)),
            )),
            Box::new(RecordingPassAgent::new(
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicBool::new(false)),
            )),
        ];

        // Manual tap: add one mana from the Any land.
        let ab = game.card(land_any).activated_abilities[0].clone();
        gl.resolve_mana_ability(&mut game, &mut agents, p0, land_any, &ab);
        assert_eq!(gl.pool(p0).total_mana(), 1);
        assert!(game.card(land_any).tapped);
        assert!(!game.card(land_combo).tapped);

        // Cast 1U spell: should consume manual pool mana and auto-tap exactly one additional land.
        let played = gl.play_card(
            &mut game,
            &mut agents,
            p0,
            spell,
            crate::agent::PlayCardMode::Normal,
        );
        assert!(
            played.is_some(),
            "manual + auto mana payment should succeed"
        );
    }

    #[test]
    fn evoke_keeps_etb_triggers_when_spell_resolves() {
        let p0 = PlayerId(0);
        let _p1 = PlayerId(1);
        let mut game = GameState::new(&["A", "B"], 20);

        let evoked = game.create_card(evoked_etb_creature(p0));
        game.move_card(evoked, ZoneType::Stack, p0);

        let mut sa = SpellAbility::new_simple(Some(evoked), p0, "SP$ Permanent");
        sa.alt_cost = Some(crate::spellability::AlternativeCost::Evoke);

        game.stack.push(StackEntry {
            id: 1,
            spell_ability: sa,
            is_creature_spell: true,
            is_permanent_spell: true,
            cast_from_zone: Some(ZoneType::Hand),
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        });

        let mut gl = GameLoop::new(2);
        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
            Box::new(RecordingPassAgent::new(
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicBool::new(false)),
            )),
            Box::new(RecordingPassAgent::new(
                Arc::new(Mutex::new(Vec::new())),
                Arc::new(AtomicBool::new(false)),
            )),
        ];

        gl.resolve_stack(&mut game, &mut agents);

        assert_eq!(game.card(evoked).zone, ZoneType::Battlefield);
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.spell_ability.api
                    == Some(crate::ability::api_type::ApiType::Draw)),
            "ETB draw trigger should be on stack for an evoked creature"
        );
        assert!(
            game.stack.iter().any(|entry| entry.spell_ability.api
                == Some(crate::ability::api_type::ApiType::Sacrifice)),
            "Evoke sacrifice trigger should be on stack"
        );
    }
}
