use std::collections::HashMap;

use forge_foundation::{PhaseType, ZoneType};

use crate::ability::effects::{self, EffectContext};
use crate::agent::{MainPhaseAction, PlayerAgent};
use crate::card::CardInstance;
use crate::combat::{self, CombatState};
use crate::cost::{self, can_pay, parse_cost, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::{self, ManaPool, basic_land_mana_atom, mana_atom_from_produced};
use crate::spellability::{build_spell_ability, SpellAbility, StackEntry};
use crate::spellability::target_restrictions;
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
        self.step_with_priority(game, agents, false);

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
        self.step_with_priority(game, agents, false);

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

    
    /// Run a priority loop until everyone passes and the stack is empty.
    /// This should be called in any phase/step where players get priority (Upkeep, Main, Combat, End).
    pub fn step_with_priority(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>], is_main_phase: bool) {
        loop {
            if game.game_over { return; }

            // 1. Process any pending triggers and put them on the stack
            self.process_triggers(game, agents);

            // 2. Give players priority
            self.priority_round(game, agents, is_main_phase);

            if game.game_over { return; }

            // 3. If stack is empty after everyone passed, the phase ends
            if game.stack.is_empty() {
                break;
            }

            // 4. Resolve top of stack (resolve_stack resolves one and gives priority)
            self.resolve_stack(game, agents);
        }
    }
pub fn step_main_phase(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        self.step_with_priority(game, agents, true);
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
    fn get_playable_cards(&self, game: &GameState, player: PlayerId, must_be_instant: bool) -> Vec<CardId> {
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
                let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
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
                let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&card.mana_cost) {
                    // Check additional costs from SP$ line (e.g. Sac<1/Creature>).
                    // Mirrors Java's CostPayment.canPayAdditionalCosts().
                    let spell_cost = Self::parse_spell_cost(&card.abilities);
                    let additional_costs_ok = if let Some(ref sc) = spell_cost {
                        sc.parts.iter().all(|part| match part {
                            CostPart::Sacrifice { type_filter, amount } => {
                                if type_filter == "CARDNAME" {
                                    true // source-sacrifice checked separately
                                } else {
                                    let targets = cost::get_sacrifice_targets(game, player, type_filter);
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
                            target_restrictions::has_candidates_in_chain(game, player, ab, Some(card_id))
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
                game.move_card(card_id, ZoneType::Battlefield, entry.spell_ability.activating_player);

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
                if game.card(card_id).zone != ZoneType::Exile && game.card(card_id).zone != ZoneType::Library && game.card(card_id).zone != ZoneType::Hand {
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

    fn resolve_spell_effect(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>], entry: &StackEntry) {
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
                    mana::auto_tap_lands(game, &mut self.mana_pools[player.index()], player, mana_cost);
                    self.mana_pools[player.index()].try_pay(mana_cost);
                }
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                }
                CostPart::Sacrifice { type_filter, amount } => {
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
                CostPart::Sacrifice { type_filter, amount } => {
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
            if game.game_over { return; }

            // Check SBA before any player gets priority
            loop {
                if !game.check_state_based_actions() { break; }
            }
            if game.game_over { return; }

            // A player can play sorcery-speed cards if:
            // - It's their own turn
            // - It's a main phase
            // - The stack is empty
            let can_play_sorcery = is_main_phase 
                && priority_player == game.active_player() 
                && game.stack.is_empty();
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
                    if !c.is_land() || !c.tapped { return false; }
                    if let Some(atom) = basic_land_mana_atom(c) {
                        pool_snapshot.has_atom(atom, 1)
                    } else {
                        false
                    }
                })
                .collect();

            let activatable = self.get_activatable_abilities(game, priority_player);

            // At instant speed (must_be_instant), tappable lands alone don't justify holding
            // priority — there's nothing instant-speed to spend the mana on. Only offer priority
            // if there are actual instant-speed spells or activatable abilities to use.
            let can_hold_priority = if must_be_instant {
                !playable.is_empty() || !untappable_lands.is_empty() || !activatable.is_empty()
            } else {
                !playable.is_empty() || !tappable_lands.is_empty() || !untappable_lands.is_empty() || !activatable.is_empty()
            };

            if !can_hold_priority {
                passed_count += 1;
                priority_player = game.next_player(priority_player);
                continue;
            }

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
                }
                MainPhaseAction::Play(card_id) => {
                    let played = self.play_card(game, agents, priority_player, card_id);
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
                    let atom_opt = {
                        let c = game.card(land_id);
                        if c.is_land() && !c.tapped { basic_land_mana_atom(c) } else { None }
                    };
                    if let Some(atom) = atom_opt {
                        game.tap(land_id);
                        self.pool_mut(priority_player).add(atom, 1);
                    }
                    passed_count = 0;
                }
                MainPhaseAction::UntapMana(land_id) => {
                    let atom_opt = {
                        let c = game.card(land_id);
                        if c.is_land() && c.tapped { basic_land_mana_atom(c) } else { None }
                    };
                    if let Some(atom) = atom_opt {
                        game.untap(land_id);
                        self.pool_mut(priority_player).remove(atom, 1);
                    }
                    passed_count = 0;
                }
                MainPhaseAction::ActivateAbility(card_id, ability_idx) => {
                    self.activate_ability(game, agents, priority_player, card_id, ability_idx);
                    passed_count = 0;
                }
            }
        }
    

}
}
