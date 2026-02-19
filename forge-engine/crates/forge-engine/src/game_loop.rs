use forge_foundation::mana::ManaAtom;
use forge_foundation::{ColorSet, ManaCost, PhaseType, ZoneType};

use crate::agent::{PlayerAgent, TargetChoice};
use crate::card::CardInstance;
use crate::combat::CombatState;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana_pool::ManaPool;
use crate::stack::StackEntry;

// ── Targeting types ─────────────────────────────────────────────────

/// What kinds of targets a spell can select.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TargetKind {
    /// Player only (e.g. "ValidTgts$ Player")
    Player,
    /// Any player or creature (e.g. "ValidTgts$ Any")
    Any,
    /// Creature with optional filter (e.g. "ValidTgts$ Creature.nonBlack")
    Creature(Option<String>),
    /// No targets
    None,
}

/// Parse ValidTgts$ from an ability string.
fn parse_valid_targets(ability: &str) -> TargetKind {
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("ValidTgts$ ") {
            let val = val.trim();
            if val.eq_ignore_ascii_case("Any") {
                return TargetKind::Any;
            }
            if val.eq_ignore_ascii_case("Player") {
                return TargetKind::Player;
            }
            if val.starts_with("Creature") {
                // e.g. "Creature.nonBlack" or just "Creature"
                let filter = val.strip_prefix("Creature").unwrap();
                if filter.is_empty() {
                    return TargetKind::Creature(None);
                }
                // Strip leading dot
                let filter = filter.strip_prefix('.').unwrap_or(filter);
                return TargetKind::Creature(Some(filter.to_string()));
            }
            // Fallback: treat as "Any" if unrecognized
            return TargetKind::Any;
        }
    }
    TargetKind::None
}

/// Check if a creature matches a filter string like "nonBlack", "nonWhite", etc.
fn matches_creature_filter(card: &CardInstance, filter: &str) -> bool {
    let lower = filter.to_ascii_lowercase();
    if let Some(color_name) = lower.strip_prefix("non") {
        let excluded = ColorSet::from_names(color_name);
        !card.color.shares_color_with(excluded)
    } else {
        // No recognized filter — match everything
        true
    }
}

/// Parse a numeric parameter from an ability string (e.g. "NumAtt$ 3" → 3).
fn parse_param(ability: &str, prefix: &str) -> Option<i32> {
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix(prefix) {
            if let Ok(n) = val.trim().parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Parse NumDmg$ value from an ability string like "NumDmg$ 3".
fn parse_num_dmg(ability: &str) -> i32 {
    parse_param(ability, "NumDmg$ ").unwrap_or(0)
}

// ── GameLoop ────────────────────────────────────────────────────────

/// Drives a complete game from setup through game over.
pub struct GameLoop {
    pub mana_pools: Vec<ManaPool>,
    pub combat: CombatState,
}

impl GameLoop {
    pub fn new(num_players: usize) -> Self {
        GameLoop {
            mana_pools: (0..num_players).map(|_| ManaPool::new()).collect(),
            combat: CombatState::new(),
        }
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

        // Beginning phase: Untap, Upkeep, Draw
        game.turn.phase = PhaseType::Untap;
        self.step_untap(game);

        game.turn.phase = PhaseType::Upkeep;
        // (No actions in simplified upkeep)

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
        // (No actions in simplified end step)

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

        // Loop: let the player take actions until they pass
        loop {
            if game.game_over {
                return;
            }

            // Resolve any pending stack entries first
            self.resolve_stack(game);
            game.check_state_based_actions();

            if game.game_over {
                return;
            }

            // Find playable cards
            let playable = self.get_playable_cards(game, active);

            if playable.is_empty() {
                break;
            }

            let agent = &mut agents[active.index()];
            let choice = agent.choose_action(active, &playable);

            match choice {
                None => break, // Pass priority
                Some(card_id) => {
                    self.play_card(game, agents, active, card_id);
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

        // Declare Attackers
        game.turn.phase = PhaseType::CombatDeclareAttackers;
        let available_attackers = self.get_available_attackers(game, active);

        if available_attackers.is_empty() {
            game.turn.phase = PhaseType::CombatEnd;
            self.combat.clear();
            return;
        }

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
        let available_blockers = self.get_available_blockers(game, defending);

        if !available_blockers.is_empty() {
            // Filter out illegal blocks (flying can only be blocked by flying/reach)
            let legal_blockers = self.filter_legal_blockers(game, &chosen_attackers, &available_blockers);

            if !legal_blockers.is_empty() {
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
        let has_first_strikers = self.combat_has_first_strikers(game);

        if has_first_strikers {
            // First Strike Damage step
            game.turn.phase = PhaseType::CombatFirstStrikeDamage;
            self.resolve_combat_damage_step(game, true);

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
        self.resolve_combat_damage_step(game, false);

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

        for &card_id in hand {
            let card = game.card(card_id);
            if card.is_land() {
                if game.player(player).can_play_land() {
                    playable.push(card_id);
                }
            } else {
                // Check if we can pay the mana cost
                let available_mana = self.calculate_available_mana(game, player);
                if available_mana.can_pay(&card.mana_cost) {
                    // For targeted spells, check that at least one valid target exists
                    let abilities = &card.abilities;
                    let mut needs_target = false;
                    let mut has_valid_target = false;

                    for ability in abilities {
                        let target_kind = parse_valid_targets(ability);
                        match target_kind {
                            TargetKind::None => {}
                            TargetKind::Player => {
                                needs_target = true;
                                let opponents: Vec<PlayerId> = game
                                    .alive_players()
                                    .into_iter()
                                    .filter(|&p| p != player)
                                    .collect();
                                if !opponents.is_empty() {
                                    has_valid_target = true;
                                }
                            }
                            TargetKind::Any => {
                                needs_target = true;
                                // Can target players or any creature on battlefield
                                let opponents: Vec<PlayerId> = game
                                    .alive_players()
                                    .into_iter()
                                    .filter(|&p| p != player)
                                    .collect();
                                if !opponents.is_empty() {
                                    has_valid_target = true;
                                } else {
                                    // Check for creatures
                                    let all_creatures = self.get_all_battlefield_creatures(game);
                                    if !all_creatures.is_empty() {
                                        has_valid_target = true;
                                    }
                                }
                            }
                            TargetKind::Creature(ref filter) => {
                                needs_target = true;
                                let valid = self.get_valid_creature_targets(game, filter.as_deref());
                                if !valid.is_empty() {
                                    has_valid_target = true;
                                }
                            }
                        }
                    }

                    if !needs_target || has_valid_target {
                        playable.push(card_id);
                    }
                }
            }
        }

        playable
    }

    /// Calculate available mana from untapped lands.
    fn calculate_available_mana(&self, game: &GameState, player: PlayerId) -> ManaPool {
        let mut pool = self.pool(player).clone();
        let lands = game.cards_in_zone(ZoneType::Battlefield, player);

        for &land_id in lands {
            let land = game.card(land_id);
            if land.is_land() && !land.tapped {
                if let Some(atom) = basic_land_mana_atom(land) {
                    pool.add(atom, 1);
                }
            }
        }

        pool
    }

    /// Play a card from hand.
    fn play_card(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
    ) {
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

            // Auto-tap lands to pay the cost
            self.auto_tap_lands(game, player, &mana_cost);

            // Check if we have an ability line that defines what this spell does
            let abilities = game.card(card_id).abilities.clone();

            // Determine targets
            let mut target_player = None;
            let mut target_card: Option<CardId> = None;

            for ability in &abilities {
                let target_kind = parse_valid_targets(ability);
                match target_kind {
                    TargetKind::None => {}
                    TargetKind::Player => {
                        let agent = &mut agents[player.index()];
                        let opponents: Vec<PlayerId> = game
                            .alive_players()
                            .into_iter()
                            .filter(|&p| p != player)
                            .collect();
                        target_player = agent.choose_target_player(player, &opponents);
                    }
                    TargetKind::Any => {
                        let opponents: Vec<PlayerId> = game
                            .alive_players()
                            .into_iter()
                            .filter(|&p| p != player)
                            .collect();
                        let valid_creatures = self.get_all_battlefield_creatures(game);
                        let agent = &mut agents[player.index()];
                        match agent.choose_target_any(player, &opponents, &valid_creatures) {
                            TargetChoice::Player(pid) => target_player = Some(pid),
                            TargetChoice::Card(cid) => target_card = Some(cid),
                            TargetChoice::None => {}
                        }
                    }
                    TargetKind::Creature(ref filter) => {
                        let valid = self.get_valid_creature_targets(game, filter.as_deref());
                        let agent = &mut agents[player.index()];
                        target_card = agent.choose_target_card(player, &valid);
                    }
                }
            }

            // Pay the mana cost from pool
            let paid = self.pool_mut(player).try_pay(&mana_cost);
            if !paid {
                return;
            }

            game.player_mut(player).spells_cast_this_turn += 1;

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
            };

            game.stack.push(entry);
            agents[player.index()].notify(&format!("Cast: {}", card_name));

            // Move spell to stack zone
            game.move_card(card_id, ZoneType::Stack, player);
        }
    }

    /// Auto-tap lands to produce the required mana.
    fn auto_tap_lands(
        &mut self,
        game: &mut GameState,
        player: PlayerId,
        cost: &ManaCost,
    ) {
        let lands: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .to_vec();

        // First, tap lands for colored requirements
        for shard in cost.shards() {
            if shard.is_x() {
                continue;
            }
            let atoms = shard.shard();
            let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;

            if color_atoms != 0 {
                for &land_id in &lands {
                    let land = game.card(land_id);
                    if land.is_land() && !land.tapped {
                        if let Some(atom) = basic_land_mana_atom(land) {
                            if (atom & color_atoms) != 0 {
                                game.tap(land_id);
                                self.pool_mut(player).add(atom, 1);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Then tap lands for generic cost
        let mut generic_needed = cost.generic_cost();
        if generic_needed > 0 {
            for &land_id in &lands {
                if generic_needed <= 0 {
                    break;
                }
                let land = game.card(land_id);
                if land.is_land() && !land.tapped {
                    if let Some(atom) = basic_land_mana_atom(land) {
                        game.tap(land_id);
                        self.pool_mut(player).add(atom, 1);
                        generic_needed -= 1;
                    }
                }
            }
        }
    }

    /// Resolve the top of the stack.
    pub fn resolve_stack(&mut self, game: &mut GameState) {
        while let Some(entry) = game.stack.pop() {
            if let Some(card_id) = entry.source {
                if entry.is_creature_spell || entry.is_permanent_spell {
                    // Permanent spell: move to battlefield
                    game.move_card(card_id, ZoneType::Battlefield, entry.controller);
                } else {
                    // Non-permanent spell: resolve effect, then move to graveyard
                    self.resolve_spell_effect(game, &entry);
                    let owner = game.card(card_id).owner;
                    game.move_card(card_id, ZoneType::Graveyard, owner);
                }
            }
        }
    }

    /// Resolve a spell effect from its ability text.
    fn resolve_spell_effect(&mut self, game: &mut GameState, entry: &StackEntry) {
        let ability = &entry.ability_text;

        if ability.contains("DealDamage") {
            let damage = parse_num_dmg(ability);
            if let Some(target_player) = entry.target_player {
                game.deal_damage_to_player(target_player, damage);
            }
            if let Some(target_card) = entry.target_card {
                // Check target is still on battlefield
                if game.card(target_card).zone == ZoneType::Battlefield {
                    game.deal_damage_to_card(target_card, damage);
                }
            }
        } else if ability.contains("Pump") {
            // Pump effect: modify power/toughness until end of turn
            let att_bonus = parse_param(ability, "NumAtt$ ").unwrap_or(0);
            let def_bonus = parse_param(ability, "NumDef$ ").unwrap_or(0);
            if let Some(target_card) = entry.target_card {
                if game.card(target_card).zone == ZoneType::Battlefield {
                    game.card_mut(target_card).power_modifier += att_bonus;
                    game.card_mut(target_card).toughness_modifier += def_bonus;
                }
            }
        } else if ability.contains("Destroy") {
            // Destroy target creature
            if let Some(target_card) = entry.target_card {
                if game.card(target_card).zone == ZoneType::Battlefield {
                    let owner = game.card(target_card).owner;
                    game.move_card(target_card, ZoneType::Graveyard, owner);
                }
            }
        } else if ability.contains("Draw") {
            // Draw cards
            let num = parse_param(ability, "NumCards$ ").unwrap_or(1);
            game.draw_cards(entry.controller, num as usize);
        }
    }

    // ── Combat helpers ──────────────────────────────────────────────

    fn get_available_attackers(&self, game: &GameState, player: PlayerId) -> Vec<CardId> {
        game.creatures_on_battlefield(player)
            .into_iter()
            .filter(|&cid| game.card(cid).can_attack())
            .collect()
    }

    fn get_available_blockers(&self, game: &GameState, player: PlayerId) -> Vec<CardId> {
        game.creatures_on_battlefield(player)
            .into_iter()
            .filter(|&cid| game.card(cid).can_block())
            .collect()
    }

    /// Filter blockers to only those that can legally block at least one attacker.
    /// A creature without flying or reach cannot block a flier.
    fn filter_legal_blockers(
        &self,
        game: &GameState,
        attackers: &[CardId],
        blockers: &[CardId],
    ) -> Vec<CardId> {
        blockers
            .iter()
            .filter(|&&blocker_id| {
                let blocker = game.card(blocker_id);
                // A blocker is legal if it can block at least one attacker
                attackers.iter().any(|&attacker_id| {
                    let attacker = game.card(attacker_id);
                    if attacker.has_flying() {
                        blocker.has_flying() || blocker.has_reach()
                    } else {
                        true
                    }
                })
            })
            .copied()
            .collect()
    }

    /// Check if any creature in combat has first strike or double strike.
    fn combat_has_first_strikers(&self, game: &GameState) -> bool {
        for &(attacker_id, _) in &self.combat.attackers {
            if game.card(attacker_id).zone != ZoneType::Battlefield {
                continue;
            }
            let card = game.card(attacker_id);
            if card.has_first_strike() || card.has_double_strike() {
                return true;
            }
        }
        for &(blocker_id, _) in &self.combat.blockers {
            if game.card(blocker_id).zone != ZoneType::Battlefield {
                continue;
            }
            let card = game.card(blocker_id);
            if card.has_first_strike() || card.has_double_strike() {
                return true;
            }
        }
        false
    }

    /// Resolve one step of combat damage.
    /// If `first_strike_only` is true, only first-strike and double-strike creatures deal damage.
    /// If false, only non-first-strike and double-strike creatures deal damage.
    fn resolve_combat_damage_step(&mut self, game: &mut GameState, first_strike_only: bool) {
        for (attacker_id, defending_player) in self.combat.attackers.clone() {
            // Check attacker is still alive
            if game.card(attacker_id).zone != ZoneType::Battlefield {
                continue;
            }

            let attacker = game.card(attacker_id);
            let attacker_has_fs = attacker.has_first_strike();
            let attacker_has_ds = attacker.has_double_strike();
            let attacker_has_trample = attacker.has_trample();
            let attacker_has_deathtouch = attacker.has_deathtouch();
            let attacker_has_lifelink = attacker.has_lifelink();
            let attacker_controller = attacker.controller;

            // Determine if this attacker deals damage in this step
            let deals_damage = if first_strike_only {
                attacker_has_fs || attacker_has_ds
            } else {
                // Regular damage step: creatures without first strike, plus double strike
                !attacker_has_fs || attacker_has_ds
            };

            if !deals_damage {
                continue;
            }

            let attacker_power = game.card(attacker_id).power();
            if attacker_power <= 0 {
                continue;
            }

            let blockers = self.combat.get_blockers_for(attacker_id);

            if blockers.is_empty() {
                // Unblocked — damage goes to defending player
                self.deal_combat_damage_to_player(
                    game,
                    defending_player,
                    attacker_power,
                    attacker_has_lifelink,
                    attacker_controller,
                );
            } else {
                // Blocked — mutual damage
                let mut remaining_damage = attacker_power;

                for &blocker_id in &blockers {
                    if remaining_damage <= 0 {
                        break;
                    }
                    // Check blocker is still alive
                    if game.card(blocker_id).zone != ZoneType::Battlefield {
                        continue;
                    }

                    let blocker_toughness = game.card(blocker_id).toughness();
                    let blocker_damage = game.card(blocker_id).damage;
                    let remaining_toughness = blocker_toughness - blocker_damage;

                    // Deathtouch: only 1 damage needed to be lethal
                    let damage_to_blocker = if attacker_has_deathtouch {
                        1.min(remaining_damage)
                    } else {
                        remaining_damage.min(remaining_toughness.max(0))
                    };

                    if damage_to_blocker > 0 {
                        self.deal_combat_damage_to_card(
                            game,
                            blocker_id,
                            damage_to_blocker,
                            attacker_has_deathtouch,
                            attacker_has_lifelink,
                            attacker_controller,
                        );
                        remaining_damage -= damage_to_blocker;
                    }

                    // Blocker damages attacker (only in the step it should deal damage)
                    let blocker_card = game.card(blocker_id);
                    let blocker_has_fs = blocker_card.has_first_strike();
                    let blocker_has_ds = blocker_card.has_double_strike();
                    let blocker_has_deathtouch = blocker_card.has_deathtouch();
                    let blocker_has_lifelink = blocker_card.has_lifelink();
                    let blocker_controller = blocker_card.controller;

                    let blocker_deals = if first_strike_only {
                        blocker_has_fs || blocker_has_ds
                    } else {
                        !blocker_has_fs || blocker_has_ds
                    };

                    if blocker_deals {
                        let blocker_power = game.card(blocker_id).power();
                        if blocker_power > 0 {
                            self.deal_combat_damage_to_card(
                                game,
                                attacker_id,
                                blocker_power,
                                blocker_has_deathtouch,
                                blocker_has_lifelink,
                                blocker_controller,
                            );
                        }
                    }
                }

                // Trample: remaining damage goes to defending player
                if attacker_has_trample && remaining_damage > 0 {
                    self.deal_combat_damage_to_player(
                        game,
                        defending_player,
                        remaining_damage,
                        attacker_has_lifelink,
                        attacker_controller,
                    );
                }
            }
        }
    }

    /// Deal combat damage to a player, handling lifelink.
    fn deal_combat_damage_to_player(
        &self,
        game: &mut GameState,
        target: PlayerId,
        amount: i32,
        lifelink: bool,
        source_controller: PlayerId,
    ) {
        if amount > 0 {
            game.deal_damage_to_player(target, amount);
            if lifelink {
                game.player_mut(source_controller).gain_life(amount);
            }
        }
    }

    /// Deal combat damage to a card, handling deathtouch and lifelink.
    fn deal_combat_damage_to_card(
        &self,
        game: &mut GameState,
        target: CardId,
        amount: i32,
        deathtouch: bool,
        lifelink: bool,
        source_controller: PlayerId,
    ) {
        if amount > 0 {
            game.deal_damage_to_card(target, amount);
            if deathtouch {
                game.card_mut(target).has_deathtouch_damage = true;
            }
            if lifelink {
                game.player_mut(source_controller).gain_life(amount);
            }
        }
    }

    // ── Targeting helpers ───────────────────────────────────────────

    /// Get all creatures on the battlefield (any player).
    fn get_all_battlefield_creatures(&self, game: &GameState) -> Vec<CardId> {
        let mut creatures = Vec::new();
        for &pid in &game.player_order {
            for &cid in game.cards_in_zone(ZoneType::Battlefield, pid) {
                if game.card(cid).is_creature() {
                    creatures.push(cid);
                }
            }
        }
        creatures
    }

    /// Get creatures matching an optional filter (e.g. "nonBlack").
    fn get_valid_creature_targets(&self, game: &GameState, filter: Option<&str>) -> Vec<CardId> {
        let all = self.get_all_battlefield_creatures(game);
        match filter {
            None => all,
            Some(f) => all
                .into_iter()
                .filter(|&cid| matches_creature_filter(game.card(cid), f))
                .collect(),
        }
    }
}

/// Determine what mana atom a basic land produces.
fn basic_land_mana_atom(card: &CardInstance) -> Option<u16> {
    if card.type_line.has_subtype("Plains") {
        Some(ManaAtom::WHITE)
    } else if card.type_line.has_subtype("Island") {
        Some(ManaAtom::BLUE)
    } else if card.type_line.has_subtype("Swamp") {
        Some(ManaAtom::BLACK)
    } else if card.type_line.has_subtype("Mountain") {
        Some(ManaAtom::RED)
    } else if card.type_line.has_subtype("Forest") {
        Some(ManaAtom::GREEN)
    } else {
        // Check card name as fallback
        match card.card_name.as_str() {
            "Plains" => Some(ManaAtom::WHITE),
            "Island" => Some(ManaAtom::BLUE),
            "Swamp" => Some(ManaAtom::BLACK),
            "Mountain" => Some(ManaAtom::RED),
            "Forest" => Some(ManaAtom::GREEN),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_num_dmg_test() {
        assert_eq!(
            parse_num_dmg(
                "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ test"
            ),
            3
        );
    }

    #[test]
    fn basic_land_detection() {
        let card = CardInstance::new(
            CardId(0),
            "Mountain".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Basic Land - Mountain"),
            ManaCost::no_cost(),
            forge_foundation::ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        assert_eq!(basic_land_mana_atom(&card), Some(ManaAtom::RED));
    }

    #[test]
    fn parse_valid_targets_any() {
        assert_eq!(
            parse_valid_targets("SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3"),
            TargetKind::Any
        );
    }

    #[test]
    fn parse_valid_targets_creature_filter() {
        assert_eq!(
            parse_valid_targets("SP$ Destroy | ValidTgts$ Creature.nonBlack"),
            TargetKind::Creature(Some("nonBlack".to_string()))
        );
    }

    #[test]
    fn parse_valid_targets_creature_no_filter() {
        assert_eq!(
            parse_valid_targets("SP$ Destroy | ValidTgts$ Creature"),
            TargetKind::Creature(None)
        );
    }

    #[test]
    fn parse_valid_targets_player() {
        assert_eq!(
            parse_valid_targets("SP$ Draw | ValidTgts$ Player"),
            TargetKind::Player
        );
    }

    #[test]
    fn parse_param_test() {
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumAtt$ "),
            Some(3)
        );
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumDef$ "),
            Some(3)
        );
        assert_eq!(
            parse_param("SP$ Draw | NumCards$ 2", "NumCards$ "),
            Some(2)
        );
    }

    #[test]
    fn creature_filter_non_black() {
        let black_creature = CardInstance::new(
            CardId(0),
            "Doom".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Zombie"),
            ManaCost::parse("1 B"),
            ColorSet::BLACK,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let green_creature = CardInstance::new(
            CardId(1),
            "Bear".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        assert!(!matches_creature_filter(&black_creature, "nonBlack"));
        assert!(matches_creature_filter(&green_creature, "nonBlack"));
    }
}
