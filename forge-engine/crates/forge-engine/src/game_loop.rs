use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};

use forge_foundation::{PhaseType, ZoneType};

use crate::ability::effects::{self, EffectContext};
use crate::agent::{MainPhaseAction, PlayerAgent};
use crate::card::CardInstance;
use crate::combat::{self, CombatState};
use crate::cost::{self, can_pay, parse_cost, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::{self, basic_land_mana_atom, mana_atom_from_produced, ManaPool};
use crate::spellability::target_restrictions;
use crate::spellability::{build_spell_ability, SpellAbility, StackEntry};
use crate::staticability::layer::apply_continuous_effects;
use crate::trigger::handler::TriggerHandler;
use crate::trigger::parse_pipe_params;

// ── GameLoop ────────────────────────────────────────────────────────

/// Drives a complete game from setup through game over.
pub struct GameLoop {
    pub mana_pools: Vec<ManaPool>,
    pub combat: CombatState,
    pub trigger_handler: TriggerHandler,
    /// Token templates keyed by their script filename stem (e.g. "r_1_1_goblin").
    /// Populated at game start by the Tauri layer; used by the Token effect handler.
    pub token_templates: HashMap<String, CardInstance>,
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
            token_templates: HashMap::new(),
        }
    }

    /// Register a token template by its script filename stem (e.g. "r_1_1_goblin").
    /// Called at game start by the Tauri layer for every token script in the token DB.
    pub fn register_token(&mut self, script_name: impl Into<String>, template: CardInstance) {
        self.token_templates.insert(script_name.into(), template);
    }

    pub fn pool(&self, pid: PlayerId) -> &ManaPool {
        &self.mana_pools[pid.index()]
    }

    pub fn pool_mut(&mut self, pid: PlayerId) -> &mut ManaPool {
        &mut self.mana_pools[pid.index()]
    }

    /// Set up the game: shuffle libraries, draw opening hands.
    pub fn setup(&mut self, game: &mut GameState, rng: &mut impl rand::Rng) {
        for &pid in &game.player_order.clone() {
            game.shuffle_library(pid, rng);
            game.draw_cards(pid, 7);
        }
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
        self.setup(game, rng);

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
        game.new_turn_for_player(active);

        // Snapshot + notify all agents of the turn change (display-only, before any actions)
        let turn_number = game.turn.turn_number;
        for agent in agents.iter_mut() {
            agent.snapshot_state(game, &self.mana_pools);
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

    fn run_turn_state_machine(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let mut state = TurnMachineState::Untap;
        while !game.game_over && state != TurnMachineState::Done {
            state = match state {
                TurnMachineState::Untap => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Untap,
                            emit_phase_trigger: false,
                        },
                    );
                    self.apply_turn_event(game, agents, TurnEvent::UntapStep);
                    TurnMachineState::Upkeep
                }
                TurnMachineState::Upkeep => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Upkeep,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: false,
                        },
                    );
                    TurnMachineState::Draw
                }
                TurnMachineState::Draw => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Draw,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(game, agents, TurnEvent::DrawStep);
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: false,
                        },
                    );
                    TurnMachineState::Main1
                }
                TurnMachineState::Main1 => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Main1,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: true,
                        },
                    );
                    TurnMachineState::Combat
                }
                TurnMachineState::Combat => {
                    self.apply_turn_event(game, agents, TurnEvent::CombatStep);
                    TurnMachineState::Main2
                }
                TurnMachineState::Main2 => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Main2,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: true,
                        },
                    );
                    TurnMachineState::EndOfTurn
                }
                TurnMachineState::EndOfTurn => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::EndOfTurn,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: false,
                        },
                    );
                    TurnMachineState::Cleanup
                }
                TurnMachineState::Cleanup => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Cleanup,
                            emit_phase_trigger: false,
                        },
                    );
                    self.apply_turn_event(game, agents, TurnEvent::CleanupStep);
                    self.apply_turn_event(game, agents, TurnEvent::AdvanceTurn);
                    TurnMachineState::Done
                }
                TurnMachineState::Done => TurnMachineState::Done,
            };
        }
    }

    fn apply_turn_event(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        event: TurnEvent,
    ) {
        if game.game_over {
            return;
        }
        match event {
            TurnEvent::EnterPhase {
                phase,
                emit_phase_trigger,
            } => {
                self.set_phase(game, agents, phase);
                if emit_phase_trigger {
                    self.emit_phase_trigger(game, phase);
                }
            }
            TurnEvent::PriorityWindow { is_main_phase } => {
                self.step_with_priority(game, agents, is_main_phase);
            }
            TurnEvent::UntapStep => {
                self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                    this.step_untap(game);
                });
            }
            TurnEvent::DrawStep => {
                self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                    this.step_draw(game);
                });
            }
            TurnEvent::CombatStep => {
                self.step_combat(game, agents);
            }
            TurnEvent::CleanupStep => {
                self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                    this.step_cleanup(game);
                });
            }
            TurnEvent::AdvanceTurn => {
                self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                    game.turn.next_player_turn(&game.player_order.clone());
                });
            }
        }
    }

    pub fn step_untap(&mut self, game: &mut GameState) {
        let active = game.active_player();
        game.untap_all(active);
        self.pool_mut(active).empty();
    }

    pub fn step_draw(&mut self, game: &mut GameState) {
        let active = game.active_player();
        // Skip draw on turn 1
        if game.turn.turn_number > 1 {
            game.draw_card(active);
        }
    }

    fn notify_phase_changed(&mut self, game: &GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        for agent in agents.iter_mut() {
            agent.snapshot_state(game, &self.mana_pools);
            agent.notify_phase_changed(game.turn.phase);
        }
    }

    fn notify_state_changed(&mut self, game: &GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        for agent in agents.iter_mut() {
            agent.snapshot_state(game, &self.mana_pools);
            agent.notify_state_changed();
        }
    }

    fn state_fingerprint(&self, game: &GameState) -> u64 {
        let mut hasher = DefaultHasher::new();

        hasher.write_u32(game.turn.turn_number);
        hasher.write_u32(game.turn.active_player.0);
        hasher.write_u8(game.turn.phase as u8);
        hasher.write_u32(game.turn.priority_player.0);
        hasher.write_u8(game.game_over as u8);
        hasher.write_u32(game.winner.map(|p| p.0).unwrap_or(u32::MAX));

        for p in &game.players {
            hasher.write_u32(p.id.0);
            hasher.write_i32(p.life);
            hasher.write_i32(p.poison_counters);
            hasher.write_i32(p.lands_played_this_turn);
            hasher.write_i32(p.spells_cast_this_turn);
            hasher.write_i32(p.drawn_this_turn);
        }

        for pool in &self.mana_pools {
            hasher.write_i32(pool.white);
            hasher.write_i32(pool.blue);
            hasher.write_i32(pool.black);
            hasher.write_i32(pool.red);
            hasher.write_i32(pool.green);
            hasher.write_i32(pool.colorless);
        }

        for c in &game.cards {
            hasher.write_u32(c.id.0);
            hasher.write_u32(c.owner.0);
            hasher.write_u32(c.controller.0);
            hasher.write_u8(c.zone as u8);
            hasher.write_u8(c.tapped as u8);
            hasher.write_u8(c.summoning_sick as u8);
            hasher.write_i32(c.damage);
            hasher.write_i32(c.power_modifier);
            hasher.write_i32(c.toughness_modifier);
            hasher.write_u8(c.has_deathtouch_damage as u8);
            hasher.write_u8(c.is_token as u8);
            hasher.write_u8(c.is_commander as u8);
            hasher.write_u32(c.commander_cast_count as u32);
        }

        for entry in game.stack.iter() {
            hasher.write_u32(entry.id);
            hasher.write_u32(entry.spell_ability.activating_player.0);
            hasher.write_u8(entry.is_creature_spell as u8);
            hasher.write_u8(entry.is_permanent_spell as u8);
            hasher.write_u32(entry.spell_ability.source.map(|s| s.0).unwrap_or(u32::MAX));
            hasher.write(entry.spell_ability.ability_text.as_bytes());
        }

        let mut zone_rows: Vec<String> = game
            .zones
            .iter()
            .map(|(k, z)| {
                let ids = z
                    .cards
                    .iter()
                    .map(|c| c.0.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{:?}:{}:{ids}", k.zone_type, k.owner.0)
            })
            .collect();
        zone_rows.sort_unstable();
        for row in zone_rows {
            hasher.write(row.as_bytes());
        }

        hasher.write_u32(
            self.combat
                .attacking_player
                .map(|p| p.0)
                .unwrap_or(u32::MAX),
        );
        hasher.write_u32(
            self.combat
                .defending_player
                .map(|p| p.0)
                .unwrap_or(u32::MAX),
        );
        for (attacker, defending_player) in &self.combat.attackers {
            hasher.write_u32(attacker.0);
            hasher.write_u32(defending_player.0);
        }
        for (blocker, attacker) in &self.combat.blockers {
            hasher.write_u32(blocker.0);
            hasher.write_u32(attacker.0);
        }

        hasher.finish()
    }

    fn with_shared_state_mutation<R>(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        f: impl FnOnce(&mut Self, &mut GameState, &mut [Box<dyn PlayerAgent>]) -> R,
    ) -> R {
        let before = self.state_fingerprint(game);
        let out = f(self, game, agents);
        let after = self.state_fingerprint(game);
        if before != after {
            self.notify_state_changed(game, agents);
        }
        out
    }

    fn set_phase(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        phase: PhaseType,
    ) {
        game.turn.phase = phase;
        self.notify_phase_changed(game, agents);
    }

    /// Run a priority loop until everyone passes and the stack is empty.
    /// This should be called in any phase/step where players get priority (Upkeep, Main, Combat, End).
    pub fn step_with_priority(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        is_main_phase: bool,
    ) {
        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
            game.turn.priority_player = game.active_player();
        });
        loop {
            if game.game_over {
                return;
            }

            // 1. Process any pending triggers and put them on the stack
            self.with_shared_state_mutation(game, agents, |this, game, agents| {
                this.process_triggers(game, agents);
            });

            // 2. Give players priority
            self.priority_round(game, agents, is_main_phase);

            if game.game_over {
                return;
            }

            // 3. If stack is empty after everyone passed, the phase ends
            if game.stack.is_empty() {
                break;
            }

            // 4. Resolve top of stack (resolve_stack resolves one and gives priority)
            self.with_shared_state_mutation(game, agents, |this, game, agents| {
                this.resolve_stack(game, agents);
            });
        }
    }
    pub fn step_main_phase(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        self.step_with_priority(game, agents, true);
    }

    pub fn step_combat(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        let active = game.active_player();
        let defending = game.opponent_of(active);
        self.combat.clear();
        self.combat.attacking_player = Some(active);
        self.combat.defending_player = Some(defending);

        // Begin Combat
        self.set_phase(game, agents, PhaseType::CombatBegin);
        self.emit_phase_trigger(game, PhaseType::CombatBegin);
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear();
            return;
        }

        // Recompute continuous effects before evaluating attack/block legality.
        // CantAttack / CantBlock flags are set here.
        apply_continuous_effects(game);

        // Declare Attackers
        self.set_phase(game, agents, PhaseType::CombatDeclareAttackers);
        let available_attackers = combat::get_available_attackers(game, active);

        let chosen_attackers = if available_attackers.is_empty() {
            Vec::new()
        } else {
            agents[active.index()].snapshot_state(game, &self.mana_pools);
            let agent = &mut agents[active.index()];
            agent.choose_attackers(active, &available_attackers)
        };

        // Tap attackers (Vigilance skips tapping)
        for &attacker_id in &chosen_attackers {
            if !game.card(attacker_id).has_vigilance() {
                game.tap(attacker_id);
            }
            game.card_mut(attacker_id).attacked_this_turn = true;
            self.combat.declare_attacker(attacker_id, defending);
        }
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear();
            return;
        }

        // Declare Blockers
        self.set_phase(game, agents, PhaseType::CombatDeclareBlockers);
        let available_blockers = combat::get_available_blockers(game, defending);

        if !available_blockers.is_empty() {
            // Filter out illegal blocks (flying can only be blocked by flying/reach)
            let legal_blockers =
                combat::filter_legal_blockers(game, &chosen_attackers, &available_blockers);

            if !legal_blockers.is_empty() {
                agents[defending.index()].snapshot_state(game, &self.mana_pools);
                let def_agent = &mut agents[defending.index()];
                let chosen_blockers =
                    def_agent.choose_blockers(defending, &chosen_attackers, &legal_blockers);

                for (blocker, attacker) in chosen_blockers {
                    // Validate: if attacker has flying, blocker needs flying or reach
                    let attacker_card = game.card(attacker);
                    let blocker_card = game.card(blocker);
                    if attacker_card.has_flying()
                        && !blocker_card.has_flying()
                        && !blocker_card.has_reach()
                    {
                        continue; // illegal block
                    }
                    self.combat.declare_blocker(blocker, attacker);
                }
            }
        }
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear();
            return;
        }

        // Determine if we need first strike damage step
        let has_first_strikers = self.combat.has_first_strikers(game);

        if has_first_strikers && self.combat.has_attackers() {
            // First Strike Damage step
            self.set_phase(game, agents, PhaseType::CombatFirstStrikeDamage);
            self.combat.resolve_damage_step(game, true);

            // SBA between damage steps
            loop {
                if !game.check_state_based_actions() {
                    break;
                }
            }
            if game.game_over {
                self.set_phase(game, agents, PhaseType::CombatEnd);
                self.combat.clear();
                return;
            }
            self.step_with_priority(game, agents, false);
            if game.game_over {
                self.combat.clear();
                return;
            }
        }

        // Regular Combat Damage step
        self.set_phase(game, agents, PhaseType::CombatDamage);
        self.combat.resolve_damage_step(game, false);

        // SBA after combat
        loop {
            if !game.check_state_based_actions() {
                break;
            }
        }
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear();
            return;
        }

        // End combat
        self.set_phase(game, agents, PhaseType::CombatEnd);
        self.emit_phase_trigger(game, PhaseType::CombatEnd);
        self.step_with_priority(game, agents, false);
        self.combat.clear();
    }

    pub fn step_cleanup(&self, game: &mut GameState) {
        let active = game.active_player();

        // Discard down to max hand size
        let hand_size = game.zone(ZoneType::Hand, active).len() as i32;
        let max = game.player(active).max_hand_size;
        if hand_size > max {
            let to_discard = (hand_size - max) as usize;
            for _ in 0..to_discard {
                // Discard last card in hand (simplified — no choice)
                if let Some(&card_id) = game.zone(ZoneType::Hand, active).cards.last() {
                    game.move_card(card_id, ZoneType::Graveyard, active);
                }
            }
        }

        // Remove damage and reset until-end-of-turn effects on all creatures
        for i in 0..game.cards.len() {
            if game.cards[i].zone == ZoneType::Battlefield && game.cards[i].is_creature() {
                game.cards[i].damage = 0;
                game.cards[i].power_modifier = 0;
                game.cards[i].toughness_modifier = 0;
                game.cards[i].has_deathtouch_damage = false;
            }
        }
    }

    /// Extract and parse the `Cost$` parameter from the first SP$ ability line.
    /// Mirrors Java's `SpellAbility.getPayCosts()` which returns the full cost
    /// (mana + additional costs) for a spell ability.
    fn parse_spell_cost(abilities: &[String]) -> Option<crate::cost::Cost> {
        for ability in abilities {
            let params = parse_pipe_params(ability);
            // Only process SP$ lines (spell abilities)
            if params.contains_key("SP") {
                if let Some(cost_str) = params.get("Cost") {
                    return Some(parse_cost(cost_str));
                }
            }
        }
        None
    }

    /// Get cards the active player can play.
    fn get_playable_cards(
        &self,
        game: &GameState,
        player: PlayerId,
        must_be_instant: bool,
    ) -> Vec<CardId> {
        let mut playable = Vec::new();
        let hand = game.cards_in_zone(ZoneType::Hand, player);

        // Check Command zone for commanders (with commander tax)
        let command_zone: Vec<CardId> = game.cards_in_zone(ZoneType::Command, player).to_vec();

        for card_id in command_zone {
            let card = game.card(card_id);
            if card.is_commander {
                if must_be_instant && !card.has_keyword("Flash") && !card.type_line.is_instant() {
                    continue;
                }
                let tax = card.commander_cast_count as i32 * 2;
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay_with_extra_generic(&card.mana_cost, tax) {
                    playable.push(card_id);
                }
            }
        }

        for &card_id in hand {
            let card = game.card(card_id);
            if card.is_land() {
                if !must_be_instant && game.player(player).can_play_land() {
                    playable.push(card_id);
                }
            } else {
                let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                if must_be_instant && !is_instant {
                    continue;
                }

                // Check if we can pay the mana cost
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&card.mana_cost) {
                    // Check additional costs from SP$ line (e.g. Sac<1/Creature>).
                    // Mirrors Java's CostPayment.canPayAdditionalCosts().
                    let spell_cost = Self::parse_spell_cost(&card.abilities);
                    let additional_costs_ok = if let Some(ref sc) = spell_cost {
                        sc.parts.iter().all(|part| match part {
                            CostPart::Sacrifice {
                                type_filter,
                                amount,
                            } => {
                                if type_filter == "CARDNAME" {
                                    true // source-sacrifice checked separately
                                } else {
                                    let targets =
                                        cost::get_sacrifice_targets(game, player, type_filter);
                                    (targets.len() as i32) >= *amount
                                }
                            }
                            CostPart::PayLife(life) => game.player(player).life >= *life,
                            _ => true, // Mana already checked, Tap N/A for spells
                        })
                    } else {
                        true
                    };

                    if additional_costs_ok {
                        // For targeted spells, check that at least one valid target
                        // exists across the entire SubAbility chain.
                        // Mirrors Java's setupTargets() walking the chain.
                        let all_valid = card.abilities.iter().all(|ab| {
                            target_restrictions::has_candidates_in_chain(
                                game,
                                player,
                                ab,
                                Some(card_id),
                            )
                        });
                        if all_valid {
                            playable.push(card_id);
                        }
                    }
                }
            }
        }

        playable
    }

    /// Play a card from hand. Returns the (card_id, card_name) if the card was
    /// successfully played, so the caller can emit the notification after resolution.
    fn play_card(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
    ) -> Option<(CardId, String)> {
        let card = game.card(card_id);
        let card_name = card.card_name.clone();

        if card.is_land() {
            // Play land — goes directly to battlefield
            game.move_card(card_id, ZoneType::Battlefield, player);
            game.player_mut(player).lands_played_this_turn += 1;
            agents[player.index()].notify(&format!("Played land: {}", card_name));
        } else {
            // Cast spell — tap lands for mana, put on stack, resolve
            let mana_cost = game.card(card_id).mana_cost.clone();
            let is_creature = game.card(card_id).is_creature();
            let is_permanent = game.card(card_id).is_permanent();

            // Detect commander cast from Command zone (for commander tax)
            let is_commander_cast =
                game.card(card_id).is_commander && game.card(card_id).zone == ZoneType::Command;
            let commander_tax = if is_commander_cast {
                game.card(card_id).commander_cast_count as i32 * 2
            } else {
                0
            };

            // Auto-tap lands to pay the cost
            mana::auto_tap_lands(game, self.pool_mut(player), player, &mana_cost);

            // Auto-tap extra lands for commander tax
            if commander_tax > 0 {
                mana::auto_tap_lands_generic(game, self.pool_mut(player), player, commander_tax);
            }

            // Check if we have an ability line that defines what this spell does
            let abilities = game.card(card_id).abilities.clone();

            // Pay the mana cost from pool
            let paid = self.pool_mut(player).try_pay(&mana_cost);
            if !paid {
                return None;
            }

            // Pay commander tax (extra generic mana)
            if commander_tax > 0 && !self.pool_mut(player).try_pay_extra_generic(commander_tax) {
                return None;
            }

            // Pay additional costs from SP$ line (e.g. sacrifice a creature).
            // Mirrors Java's CostPayment.payCost() iterating CostParts.
            let spell_cost = Self::parse_spell_cost(&abilities);
            if let Some(ref sc) = spell_cost {
                self.pay_additional_costs(game, agents, player, card_id, sc);
            }

            // Increment commander cast count (before moving card to stack)
            if is_commander_cast {
                game.card_mut(card_id).commander_cast_count += 1;
            }

            game.player_mut(player).spells_cast_this_turn += 1;

            // Emit SpellCast trigger
            self.trigger_handler.run_trigger(
                TriggerType::SpellCast,
                RunParams {
                    spell_card: Some(card_id),
                    spell_controller: Some(player),
                    ..Default::default()
                },
                false,
            );

            // Build SpellAbility chain and choose targets.
            // Mirrors Java's AbilityFactory.getAbility() + setupTargets().
            let ability_text = abilities.first().cloned().unwrap_or_default();
            let mut sa = build_spell_ability(game, card_id, &ability_text, player);
            sa.is_spell = true;
            sa.setup_targets(game, agents, &self.mana_pools);

            let entry = StackEntry {
                id: 0,
                spell_ability: sa,
                is_creature_spell: is_creature,
                is_permanent_spell: is_permanent,
            };

            game.stack.push(entry);
            agents[player.index()].notify(&format!("Cast: {}", card_name));

            // Move spell to stack zone
            game.move_card(card_id, ZoneType::Stack, player);
        }

        Some((card_id, card_name))
    }

    /// Resolve the top item of the stack.
    /// Does NOT run a priority_round — the caller (step_with_priority) handles that
    /// both before calling this and after (via the outer loop) so players get priority
    /// between each resolved item.
    pub fn resolve_stack(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        if game.stack.is_empty() {
            return;
        }

        let entry = game.stack.pop().unwrap();

        if entry.spell_ability.is_trigger || entry.spell_ability.is_activated {
            // Triggered/activated ability: resolve the effect
            self.resolve_spell_effect(game, agents, &entry);
        } else if let Some(card_id) = entry.spell_ability.source {
            if entry.is_creature_spell || entry.is_permanent_spell {
                // Permanent spell: move to battlefield
                let origin = game.card(card_id).zone;
                game.move_card(
                    card_id,
                    ZoneType::Battlefield,
                    entry.spell_ability.activating_player,
                );

                // Register triggers for the new permanent
                self.trigger_handler.register_active_trigger(game, card_id);

                // Emit ChangesZone trigger (ETB)
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    card_id,
                    origin,
                    ZoneType::Battlefield,
                );
            } else {
                // Non-permanent spell: resolve effect, then move to graveyard
                self.resolve_spell_effect(game, agents, &entry);
                let owner = game.card(card_id).owner;
                // Only move to graveyard if it's still in stack zone
                if game.card(card_id).zone != ZoneType::Exile
                    && game.card(card_id).zone != ZoneType::Library
                    && game.card(card_id).zone != ZoneType::Hand
                {
                    game.move_card(card_id, ZoneType::Graveyard, owner);
                }
            }
        }

        // Continuous effects might change after resolution
        apply_continuous_effects(game);
        game.check_state_based_actions();

        // Process triggers that may have fired during resolution (puts them on stack
        // so the outer step_with_priority loop can give priority before resolving them)
        self.process_triggers(game, agents);
    }

    fn resolve_spell_effect(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        entry: &StackEntry,
    ) {
        // Walk the SpellAbility chain: resolve each node's effect, propagating
        // the parent SA's chosen target card so sub-abilities can resolve
        // `Defined$ ParentTarget`. Mirrors Java's resolveApiAbility() + resolveSubAbilities().
        let mut parent_target_card: Option<CardId> = None;
        let mut current = Some(&entry.spell_ability);
        while let Some(sa) = current {
            self.resolve_single_effect(game, agents, sa, parent_target_card);
            // This SA's target card becomes the parent context for the next sub-ability.
            parent_target_card = sa.target_chosen.target_card;
            current = sa.get_sub_ability();
        }
    }

    /// Resolve a single effect line by delegating to the effects module.
    fn resolve_single_effect(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        sa: &SpellAbility,
        parent_target_card: Option<CardId>,
    ) {
        let mut ctx = EffectContext {
            game,
            agents,
            trigger_handler: &mut self.trigger_handler,
            token_templates: &self.token_templates,
            mana_pools: &mut self.mana_pools,
            parent_target_card,
        };
        effects::resolve_effect(&mut ctx, sa);
    }

    // ── Trigger helpers ────────────────────────────────────────────

    /// Emit a phase trigger event.
    fn emit_phase_trigger(&mut self, game: &GameState, phase: PhaseType) {
        let active = game.active_player();
        self.trigger_handler.run_trigger(
            TriggerType::Phase,
            RunParams {
                phase: Some(phase),
                player: Some(active),
                ..Default::default()
            },
            false,
        );
    }

    /// Process pending triggers: drain the waiting queue, put abilities on stack, resolve.
    /// Mirrors Java's runWaitingTriggers() called between stack resolution windows.
    fn process_triggers(&mut self, game: &mut GameState, _agents: &mut [Box<dyn PlayerAgent>]) {
        let entries = self.trigger_handler.run_waiting_triggers(game);
        for entry in entries {
            game.stack.push(entry);
        }
    }

    // ── Activated Ability helpers ───────────────────────────────────

    /// Find all activatable abilities for a player on the battlefield.
    fn get_activatable_abilities(
        &self,
        game: &GameState,
        player: PlayerId,
    ) -> Vec<(CardId, usize)> {
        let mut result = Vec::new();
        let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
        let battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();

        for card_id in battlefield {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if can_pay(&ab.cost, game, &available_mana, card_id, player) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        result
    }

    /// Activate an ability on a permanent.
    fn activate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ability_idx: usize,
    ) {
        // Clone the ability data we need before mutating game
        let ab = {
            let card = game.card(card_id);
            card.activated_abilities
                .iter()
                .find(|a| a.ability_index == ability_idx)
                .cloned()
        };

        let ab = match ab {
            Some(ab) => ab,
            None => return,
        };

        if ab.is_mana_ability {
            self.resolve_mana_ability(game, agents, player, card_id, &ab);
        } else {
            self.activate_ability_on_stack(game, agents, player, card_id, &ab);
        }
    }

    /// Pay the cost parts of an activated ability (tap, mana, life, sacrifice).
    /// Mirrors Java's `CostPayment.payCost()` iterating over `CostPart`s.
    fn pay_ability_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        cost: &crate::cost::Cost,
    ) {
        for part in &cost.parts {
            match part {
                CostPart::Tap => {
                    game.tap(card_id);
                }
                CostPart::Mana(mana_cost) => {
                    mana::auto_tap_lands(
                        game,
                        &mut self.mana_pools[player.index()],
                        player,
                        mana_cost,
                    );
                    self.mana_pools[player.index()].try_pay(mana_cost);
                }
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        game.move_card(card_id, ZoneType::Graveyard, owner);
                    } else {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
            }
        }
    }

    /// Pay additional costs from an SP$ ability line (non-mana cost parts only).
    /// Used during spell casting for costs like `Sac<1/Creature>`.
    /// Mirrors Java's `CostPayment.payCost()` for spell abilities.
    fn pay_additional_costs(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        _card_id: CardId,
        spell_cost: &crate::cost::Cost,
    ) {
        for part in &spell_cost.parts {
            match part {
                // Mana is already paid by play_card's main mana payment flow
                CostPart::Mana(_) | CostPart::Tap => {}
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter != "CARDNAME" {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
            }
        }
    }

    /// Pay a sacrifice cost by prompting the agent to choose targets.
    /// Mirrors Java's `CostSacrifice.doListPayment()` which calls
    /// `GameAction.sacrifice()`.
    fn pay_sacrifice_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid = cost::get_sacrifice_targets(game, player, type_filter);
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Graveyard, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Battlefield,
                    ZoneType::Graveyard,
                );
            }
        }
    }

    /// Resolve a mana ability immediately (no stack).
    fn resolve_mana_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) {
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Produce mana
        if let Some(produced) = ab.params.get("Produced") {
            if let Some(atom) = mana_atom_from_produced(produced) {
                self.pool_mut(player).add(atom, 1);
            }
        }
    }

    /// Activate a non-mana ability: choose targets, pay costs, put on stack.
    fn activate_ability_on_stack(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) {
        let ability_text = ab.ability_text.clone();

        // Build SpellAbility and choose targets
        let mut sa = SpellAbility::new_simple(Some(card_id), player, &ability_text);
        sa.is_activated = true;
        sa.setup_targets(game, agents, &self.mana_pools);

        // Pay costs
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: false,
            is_permanent_spell: false,
        };
        game.stack.push(entry);
        agents[player.index()].notify(&format!("Activated ability of {}", card_name));
    }

    /// Give players priority to take actions (play cards, activate abilities).
    /// Returns when all players pass in succession.
    pub fn priority_round(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        is_main_phase: bool,
    ) {
        let mut priority_player = game.active_player();
        let mut passed_count = 0;
        let num_players = game.players.len();

        while passed_count < num_players {
            if game.game_over {
                return;
            }
            self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                game.turn.priority_player = priority_player;
            });

            // Check SBA before any player gets priority
            loop {
                if !game.check_state_based_actions() {
                    break;
                }
            }
            if game.game_over {
                return;
            }

            // A player can play sorcery-speed cards if:
            // - It's their own turn
            // - It's a main phase
            // - The stack is empty
            let can_play_sorcery =
                is_main_phase && priority_player == game.active_player() && game.stack.is_empty();
            let must_be_instant = !can_play_sorcery;

            let playable = self.get_playable_cards(game, priority_player, must_be_instant);

            let tappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, priority_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let c = game.card(cid);
                    c.is_land() && !c.tapped
                })
                .collect();

            let pool_snapshot = self.pool(priority_player).clone();
            let untappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, priority_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let c = game.card(cid);
                    if !c.is_land() || !c.tapped {
                        return false;
                    }
                    if let Some(atom) = basic_land_mana_atom(c) {
                        pool_snapshot.has_atom(atom, 1)
                    } else {
                        false
                    }
                })
                .collect();

            let activatable = self.get_activatable_abilities(game, priority_player);

            agents[priority_player.index()].snapshot_state(game, &self.mana_pools);
            let action = agents[priority_player.index()].choose_action(
                priority_player,
                &playable,
                &tappable_lands,
                &untappable_lands,
                &activatable,
            );

            match action {
                MainPhaseAction::Pass => {
                    passed_count += 1;
                    priority_player = game.next_player(priority_player);
                    self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                        game.turn.priority_player = priority_player;
                    });
                }
                MainPhaseAction::Play(card_id) => {
                    if !playable.contains(&card_id) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: unplayable card");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    let played =
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.play_card(game, agents, priority_player, card_id)
                        });
                    if let Some((played_id, played_name)) = played {
                        let set_code = game.card(played_id).set_code.clone().unwrap_or_default();
                        for agent in agents.iter_mut() {
                            agent.snapshot_state(game, &self.mana_pools);
                            agent.notify_card_played(priority_player, played_id, &played_name, &set_code);
                        }
                    }
                    passed_count = 0;
                }
                MainPhaseAction::ActivateMana(land_id) => {
                    if !tappable_lands.contains(&land_id) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: land can't tap for mana");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                        let atom_opt = {
                            let c = game.card(land_id);
                            if c.is_land() && !c.tapped {
                                basic_land_mana_atom(c)
                            } else {
                                None
                            }
                        };
                        if let Some(atom) = atom_opt {
                            game.tap(land_id);
                            this.pool_mut(priority_player).add(atom, 1);
                        }
                    });
                    passed_count = 0;
                }
                MainPhaseAction::UntapMana(land_id) => {
                    if !untappable_lands.contains(&land_id) {
                        agents[priority_player.index()].notify(
                            "Illegal action ignored: land can't be untapped for mana rollback",
                        );
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                        let atom_opt = {
                            let c = game.card(land_id);
                            if c.is_land() && c.tapped {
                                basic_land_mana_atom(c)
                            } else {
                                None
                            }
                        };
                        if let Some(atom) = atom_opt {
                            game.untap(land_id);
                            this.pool_mut(priority_player).remove(atom, 1);
                        }
                    });
                    passed_count = 0;
                }
                MainPhaseAction::ActivateAbility(card_id, ability_idx) => {
                    if !activatable.contains(&(card_id, ability_idx)) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: ability not activatable");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    self.with_shared_state_mutation(game, agents, |this, game, agents| {
                        this.activate_ability(game, agents, priority_player, card_id, ability_idx);
                    });
                    passed_count = 0;
                }
            }
        }
        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
            game.turn.priority_player = game.active_player();
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};
    use rand::SeedableRng;

    use crate::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
    use crate::card::CardInstance;

    use super::*;

    struct RecordingPassAgent {
        phases_seen: Arc<Mutex<Vec<PhaseType>>>,
        bad_priority_seen: Arc<AtomicBool>,
        last_phase: Option<PhaseType>,
        last_priority: Option<PlayerId>,
    }

    struct InvalidPlayAgent;

    impl PlayerAgent for InvalidPlayAgent {
        fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
            true
        }

        fn choose_action(
            &mut self,
            _player: PlayerId,
            _playable: &[CardId],
            _tappable_lands: &[CardId],
            _untappable_lands: &[CardId],
            _activatable: &[(CardId, usize)],
        ) -> MainPhaseAction {
            MainPhaseAction::Play(CardId(u32::MAX))
        }

        fn choose_attackers(&mut self, _player: PlayerId, _available: &[CardId]) -> Vec<CardId> {
            Vec::new()
        }

        fn choose_blockers(
            &mut self,
            _player: PlayerId,
            _attackers: &[CardId],
            _available_blockers: &[CardId],
        ) -> Vec<(CardId, CardId)> {
            Vec::new()
        }

        fn choose_target_player(
            &mut self,
            _player: PlayerId,
            valid: &[PlayerId],
        ) -> Option<PlayerId> {
            valid.first().copied()
        }

        fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
            valid.first().copied()
        }

        fn choose_target_any(
            &mut self,
            _player: PlayerId,
            valid_players: &[PlayerId],
            valid_cards: &[CardId],
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

        fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
            true
        }

        fn choose_action(
            &mut self,
            player: PlayerId,
            _playable: &[CardId],
            _tappable_lands: &[CardId],
            _untappable_lands: &[CardId],
            _activatable: &[(CardId, usize)],
        ) -> MainPhaseAction {
            if self.last_priority != Some(player) {
                self.bad_priority_seen.store(true, Ordering::SeqCst);
            }
            if let Some(phase) = self.last_phase {
                self.phases_seen.lock().unwrap().push(phase);
            }
            MainPhaseAction::Pass
        }

        fn choose_attackers(&mut self, _player: PlayerId, _available: &[CardId]) -> Vec<CardId> {
            Vec::new()
        }

        fn choose_blockers(
            &mut self,
            _player: PlayerId,
            _attackers: &[CardId],
            _available_blockers: &[CardId],
        ) -> Vec<(CardId, CardId)> {
            Vec::new()
        }

        fn choose_target_player(
            &mut self,
            _player: PlayerId,
            valid: &[PlayerId],
        ) -> Option<PlayerId> {
            valid.first().copied()
        }

        fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
            valid.first().copied()
        }

        fn choose_target_any(
            &mut self,
            _player: PlayerId,
            valid_players: &[PlayerId],
            valid_cards: &[CardId],
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

    fn zero_cost_instant(owner: PlayerId) -> CardInstance {
        CardInstance::new(
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

        assert!(all_phases.contains(&PhaseType::Draw));
        assert!(all_phases.contains(&PhaseType::CombatBegin));
        assert!(all_phases.contains(&PhaseType::CombatDeclareAttackers));
        assert!(all_phases.contains(&PhaseType::CombatDeclareBlockers));
        assert!(all_phases.contains(&PhaseType::CombatDamage));
        assert!(all_phases.contains(&PhaseType::CombatEnd));
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
}
