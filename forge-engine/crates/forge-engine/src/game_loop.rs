use std::collections::HashMap;

use forge_foundation::{PhaseType, ZoneType};

use crate::ability::effects::{self, EffectContext};
use crate::agent::{MainPhaseAction, PlayerAgent};
use crate::card::CardInstance;
use crate::combat::{self, CombatState};
use crate::cost::{can_pay, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::{self, ManaPool, basic_land_mana_atom, mana_atom_from_produced};
use crate::spellability::StackEntry;
use crate::spellability::targeting;
use crate::staticability::layer::apply_continuous_effects;
use crate::trigger::parse_pipe_params;
use crate::trigger::handler::TriggerHandler;

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

        // Beginning phase: Untap, Upkeep, Draw
        game.turn.phase = PhaseType::Untap;
        self.step_untap(game);

        game.turn.phase = PhaseType::Upkeep;
        self.emit_phase_trigger(game, PhaseType::Upkeep);
        self.process_triggers(game, agents);

        game.turn.phase = PhaseType::Draw;
        self.step_draw(game);

        // Main Phase 1
        game.turn.phase = PhaseType::Main1;
        self.step_main_phase(game, agents);

        if game.game_over {
            return;
        }

        // Combat Phase
        self.step_combat(game, agents);

        if game.game_over {
            return;
        }

        // Main Phase 2
        game.turn.phase = PhaseType::Main2;
        self.step_main_phase(game, agents);

        if game.game_over {
            return;
        }

        // End phase
        game.turn.phase = PhaseType::EndOfTurn;
        self.emit_phase_trigger(game, PhaseType::EndOfTurn);
        self.process_triggers(game, agents);

        game.turn.phase = PhaseType::Cleanup;
        self.step_cleanup(game);

        // Advance to next player's turn
        game.turn.next_player_turn(&game.player_order.clone());
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

    pub fn step_main_phase(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let active = game.active_player();

        // Card info saved from a play action so we can notify after resolution.
        let mut pending_play: Option<(CardId, String)> = None;

        // Loop: let the player take actions until they pass
        loop {
            if game.game_over {
                return;
            }

            // Resolve any pending stack entries first
            self.resolve_stack(game, agents);
            // Recompute continuous effects after resolution (a permanent may
            // have entered or left the battlefield).
            apply_continuous_effects(game);
            game.check_state_based_actions();

            if game.game_over {
                return;
            }

            // If a card was just played, the stack has now resolved and SBAs
            // have been checked — snapshot + notify with the post-resolution state.
            if let Some((played_id, played_name)) = pending_play.take() {
                for agent in agents.iter_mut() {
                    agent.snapshot_state(game, &self.mana_pools);
                }
                for agent in agents.iter_mut() {
                    agent.notify_card_played(active, played_id, &played_name);
                }
            }

            // Find playable hand cards
            let playable = self.get_playable_cards(game, active);

            // Find untapped lands the player can manually tap for mana
            let tappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, active)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let c = game.card(cid);
                    c.is_land() && !c.tapped
                })
                .collect();

            // Find tapped lands whose mana is still in the pool (can be untapped)
            let pool_snapshot = self.pool(active).clone();
            let untappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, active)
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

            // Find activatable abilities on battlefield permanents
            let activatable = self.get_activatable_abilities(game, active);

            // Auto-break only when truly nothing can be done
            if playable.is_empty()
                && tappable_lands.is_empty()
                && untappable_lands.is_empty()
                && activatable.is_empty()
            {
                break;
            }

            agents[active.index()].snapshot_state(game, &self.mana_pools);
            let agent = &mut agents[active.index()];
            let action = agent.choose_action(
                active,
                &playable,
                &tappable_lands,
                &untappable_lands,
                &activatable,
            );

            match action {
                MainPhaseAction::Pass => break,
                MainPhaseAction::Play(card_id) => {
                    pending_play = self.play_card(game, agents, active, card_id);
                }
                MainPhaseAction::ActivateMana(land_id) => {
                    // Compute the mana atom before mutably borrowing game
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
                        self.pool_mut(active).add(atom, 1);
                    }
                }
                MainPhaseAction::UntapMana(land_id) => {
                    // Compute the mana atom before mutably borrowing game
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
                        self.pool_mut(active).remove(atom, 1);
                    }
                }
                MainPhaseAction::ActivateAbility(card_id, ability_idx) => {
                    self.activate_ability(game, agents, active, card_id, ability_idx);
                }
            }
        }
    }

    pub fn step_combat(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let active = game.active_player();
        let defending = game.opponent_of(active);
        self.combat.clear();
        self.combat.attacking_player = Some(active);
        self.combat.defending_player = Some(defending);

        // Begin Combat
        game.turn.phase = PhaseType::CombatBegin;

        // Recompute continuous effects before evaluating attack/block legality.
        // CantAttack / CantBlock flags are set here.
        apply_continuous_effects(game);

        // Declare Attackers
        game.turn.phase = PhaseType::CombatDeclareAttackers;
        let available_attackers = combat::get_available_attackers(game, active);

        if available_attackers.is_empty() {
            game.turn.phase = PhaseType::CombatEnd;
            self.combat.clear();
            return;
        }

        agents[active.index()].snapshot_state(game, &self.mana_pools);
        let agent = &mut agents[active.index()];
        let chosen_attackers = agent.choose_attackers(active, &available_attackers);

        if chosen_attackers.is_empty() {
            game.turn.phase = PhaseType::CombatEnd;
            self.combat.clear();
            return;
        }

        // Tap attackers (Vigilance skips tapping)
        for &attacker_id in &chosen_attackers {
            if !game.card(attacker_id).has_vigilance() {
                game.tap(attacker_id);
            }
            game.card_mut(attacker_id).attacked_this_turn = true;
            self.combat.declare_attacker(attacker_id, defending);
        }

        // Declare Blockers
        game.turn.phase = PhaseType::CombatDeclareBlockers;
        let available_blockers = combat::get_available_blockers(game, defending);

        if !available_blockers.is_empty() {
            // Filter out illegal blocks (flying can only be blocked by flying/reach)
            let legal_blockers = combat::filter_legal_blockers(game, &chosen_attackers, &available_blockers);

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

        // Determine if we need first strike damage step
        let has_first_strikers = self.combat.has_first_strikers(game);

        if has_first_strikers {
            // First Strike Damage step
            game.turn.phase = PhaseType::CombatFirstStrikeDamage;
            self.combat.resolve_damage_step(game, true);

            // SBA between damage steps
            loop {
                if !game.check_state_based_actions() {
                    break;
                }
            }
            if game.game_over {
                game.turn.phase = PhaseType::CombatEnd;
                self.combat.clear();
                return;
            }
        }

        // Regular Combat Damage step
        game.turn.phase = PhaseType::CombatDamage;
        self.combat.resolve_damage_step(game, false);

        // SBA after combat
        loop {
            if !game.check_state_based_actions() {
                break;
            }
        }

        // End combat
        game.turn.phase = PhaseType::CombatEnd;
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

    /// Get cards the active player can play.
    fn get_playable_cards(&self, game: &GameState, player: PlayerId) -> Vec<CardId> {
        let mut playable = Vec::new();
        let hand = game.cards_in_zone(ZoneType::Hand, player);

        // Check Command zone for commanders (with commander tax)
        let command_zone: Vec<CardId> = game.cards_in_zone(ZoneType::Command, player).to_vec();
        for card_id in command_zone {
            let card = game.card(card_id);
            if card.is_commander {
                let tax = card.commander_cast_count as i32 * 2;
                let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay_with_extra_generic(&card.mana_cost, tax) {
                    playable.push(card_id);
                }
            }
        }

        for &card_id in hand {
            let card = game.card(card_id);
            if card.is_land() {
                if game.player(player).can_play_land() {
                    playable.push(card_id);
                }
            } else {
                // Check if we can pay the mana cost
                let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&card.mana_cost) {
                    // For targeted spells, check that at least one valid target exists
                    let all_valid = card.abilities.iter().all(|ab| {
                        targeting::has_valid_target(game, player, ab)
                    });
                    if all_valid {
                        playable.push(card_id);
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
            let is_commander_cast = game.card(card_id).is_commander
                && game.card(card_id).zone == ZoneType::Command;
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

            // Determine targets (use first ability's targeting)
            let (target_player, target_card) = if let Some(ability) = abilities.first() {
                targeting::choose_targets(game, agents, &self.mana_pools, player, ability)
            } else {
                (None, None)
            };

            // Pay the mana cost from pool
            let paid = self.pool_mut(player).try_pay(&mana_cost);
            if !paid {
                return None;
            }

            // Pay commander tax (extra generic mana)
            if commander_tax > 0 && !self.pool_mut(player).try_pay_extra_generic(commander_tax) {
                return None;
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

            // Put on stack and resolve immediately (simplified — no priority passing)
            let ability_text = abilities.first().cloned().unwrap_or_default();

            let entry = StackEntry {
                id: 0,
                source: Some(card_id),
                controller: player,
                ability_text,
                is_creature_spell: is_creature,
                is_permanent_spell: is_permanent,
                target_player,
                target_card,
                is_triggered_ability: false,
                is_activated_ability: false,
                trigger_source: None,
                trigger_index: None,
            };

            game.stack.push(entry);
            agents[player.index()].notify(&format!("Cast: {}", card_name));

            // Move spell to stack zone
            game.move_card(card_id, ZoneType::Stack, player);
        }

        Some((card_id, card_name))
    }

    /// Resolve the top of the stack.
    pub fn resolve_stack(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        while let Some(entry) = game.stack.pop() {
            if entry.is_triggered_ability || entry.is_activated_ability {
                // Triggered/activated ability: resolve the effect
                self.resolve_spell_effect(game, agents, &entry);
                continue;
            }

            if let Some(card_id) = entry.source {
                if entry.is_creature_spell || entry.is_permanent_spell {
                    // Permanent spell: move to battlefield
                    let origin = game.card(card_id).zone;
                    game.move_card(card_id, ZoneType::Battlefield, entry.controller);

                    // Register triggers for the new permanent
                    self.trigger_handler.register_active_trigger(game, card_id);

                    // Emit ChangesZone trigger (ETB)
                    self.trigger_handler.run_trigger(
                        TriggerType::ChangesZone,
                        RunParams {
                            card: Some(card_id),
                            origin: Some(origin),
                            destination: Some(ZoneType::Battlefield),
                            ..Default::default()
                        },
                        false,
                    );
                } else {
                    // Non-permanent spell: resolve effect, then move to graveyard
                    self.resolve_spell_effect(game, agents, &entry);
                    let owner = game.card(card_id).owner;
                    game.move_card(card_id, ZoneType::Graveyard, owner);
                }
            }
        }

        // Process any triggers that were queued during resolution
        self.process_triggers(game, agents);
    }

    /// Resolve a spell effect from its ability text.
    /// Handles both SP$ (spell) and DB$ (triggered/sub-ability) formats.
    fn resolve_spell_effect(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>], entry: &StackEntry) {
        self.resolve_single_effect(game, agents, &entry.ability_text, entry);

        // Handle SubAbility$ chain (mirrors Java's getSubAbility() linked list)
        let params = parse_pipe_params(&entry.ability_text);
        if let Some(sub_svar_name) = params.get("SubAbility") {
            if let Some(source_card) = entry.source {
                let sub_text = game.card(source_card).svars.get(sub_svar_name).cloned();
                if let Some(sub_text) = sub_text {
                    let sub_entry = StackEntry {
                        id: 0,
                        source: entry.source,
                        controller: entry.controller,
                        ability_text: sub_text,
                        is_creature_spell: false,
                        is_permanent_spell: false,
                        target_player: entry.target_player,
                        target_card: entry.target_card,
                        is_triggered_ability: entry.is_triggered_ability,
                        is_activated_ability: entry.is_activated_ability,
                        trigger_source: entry.trigger_source,
                        trigger_index: entry.trigger_index,
                    };
                    self.resolve_spell_effect(game, agents, &sub_entry);
                }
            }
        }
    }

    /// Resolve a single effect line by delegating to the effects module.
    fn resolve_single_effect(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        ability: &str,
        entry: &StackEntry,
    ) {
        let mut ctx = EffectContext {
            game,
            agents,
            trigger_handler: &mut self.trigger_handler,
            token_templates: &self.token_templates,
            mana_pools: &mut self.mana_pools,
        };
        effects::resolve_effect(&mut ctx, ability, entry);
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
    fn process_triggers(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        // Loop in case triggers trigger more triggers
        let mut safety = 0;
        loop {
            let entries = self.trigger_handler.run_waiting_triggers(game);
            if entries.is_empty() {
                break;
            }

            for entry in entries {
                game.stack.push(entry);
            }

            // Resolve triggered abilities on the stack
            while let Some(entry) = game.stack.pop() {
                if entry.is_triggered_ability {
                    self.resolve_spell_effect(game, agents, &entry);
                }
            }

            safety += 1;
            if safety > 100 {
                break; // prevent infinite loops
            }
        }
    }

    // ── Activated Ability helpers ───────────────────────────────────

    /// Find all activatable abilities for a player on the battlefield.
    /// Returns (card_id, ability_index) pairs.
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
            self.resolve_mana_ability(game, player, card_id, &ab);
        } else {
            self.activate_ability_on_stack(game, agents, player, card_id, &ab);
        }
    }

    /// Pay the cost parts of an activated ability (tap, mana, life, sacrifice).
    fn pay_ability_cost(
        &mut self,
        game: &mut GameState,
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
                    mana::auto_tap_lands(game, &mut self.mana_pools[player.index()], player, mana_cost);
                    self.mana_pools[player.index()].try_pay(mana_cost);
                }
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                }
                CostPart::Sacrifice { type_filter, .. } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        game.move_card(card_id, ZoneType::Graveyard, owner);
                    }
                }
            }
        }
    }

    /// Resolve a mana ability immediately (no stack).
    fn resolve_mana_ability(
        &mut self,
        game: &mut GameState,
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) {
        self.pay_ability_cost(game, player, card_id, &ab.cost);

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

        // Determine targets
        let (target_player, target_card) =
            targeting::choose_targets(game, agents, &self.mana_pools, player, &ability_text);

        // Pay costs
        self.pay_ability_cost(game, player, card_id, &ab.cost);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let entry = StackEntry {
            id: 0,
            source: Some(card_id),
            controller: player,
            ability_text,
            is_creature_spell: false,
            is_permanent_spell: false,
            target_player,
            target_card,
            is_triggered_ability: false,
            is_activated_ability: true,
            trigger_source: None,
            trigger_index: None,
        };
        game.stack.push(entry);
        agents[player.index()].notify(&format!("Activated ability of {}", card_name));
    }
}

