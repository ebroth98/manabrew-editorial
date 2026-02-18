use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, PhaseType, ZoneType};

use crate::agent::PlayerAgent;
use crate::card::CardInstance;
use crate::combat::CombatState;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana_pool::ManaPool;
use crate::stack::StackEntry;

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

            // Tap lands for mana (auto-tap available lands)
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

        for &attacker_id in &chosen_attackers {
            game.tap(attacker_id);
            game.card_mut(attacker_id).attacked_this_turn = true;
            self.combat.declare_attacker(attacker_id, defending);
        }

        // Declare Blockers
        game.turn.phase = PhaseType::CombatDeclareBlockers;
        let available_blockers = self.get_available_blockers(game, defending);

        if !available_blockers.is_empty() {
            let def_agent = &mut agents[defending.index()];
            let chosen_blockers =
                def_agent.choose_blockers(defending, &chosen_attackers, &available_blockers);

            for (blocker, attacker) in chosen_blockers {
                self.combat.declare_blocker(blocker, attacker);
            }
        }

        // Combat Damage (skip first strike for simplicity)
        game.turn.phase = PhaseType::CombatDamage;
        self.resolve_combat_damage(game);

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

        // Remove damage from all creatures
        for i in 0..game.cards.len() {
            if game.cards[i].zone == ZoneType::Battlefield && game.cards[i].is_creature() {
                game.cards[i].damage = 0;
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
                // First, calculate available mana from untapped lands
                let available_mana = self.calculate_available_mana(game, player);
                if available_mana.can_pay(&card.mana_cost) {
                    playable.push(card_id);
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
                // Determine what color this land produces based on basic land types
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

            // Determine target for damage spells
            let mut target_player = None;
            let target_card: Option<CardId> = None;

            for ability in &abilities {
                if ability.contains("DealDamage") && ability.contains("ValidTgts$") {
                    // This is a damage spell — ask for target
                    let agent = &mut agents[player.index()];
                    let opponents: Vec<PlayerId> = game
                        .alive_players()
                        .into_iter()
                        .filter(|&p| p != player)
                        .collect();

                    // Check if it can target creatures too
                    if ability.contains("Any") || ability.contains("Creature") {
                        // Can target anything — for now just let agent choose player
                        target_player = agent.choose_target_player(player, &opponents);
                    } else {
                        target_player = agent.choose_target_player(player, &opponents);
                    }
                }
            }

            // Pay the mana cost from pool
            let paid = self.pool_mut(player).try_pay(&mana_cost);
            if !paid {
                // Shouldn't happen if get_playable_cards is correct
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
                // Find an untapped land that produces this color
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

        // Parse "DealDamage" effects
        if ability.contains("DealDamage") {
            let damage = parse_num_dmg(ability);

            if let Some(target_player) = entry.target_player {
                game.deal_damage_to_player(target_player, damage);
            }
            if let Some(target_card) = entry.target_card {
                game.deal_damage_to_card(target_card, damage);
            }
        }
    }

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

    fn resolve_combat_damage(&mut self, game: &mut GameState) {
        for (attacker_id, defending_player) in self.combat.attackers.clone() {
            let blockers = self.combat.get_blockers_for(attacker_id);

            if blockers.is_empty() {
                // Unblocked — damage goes to defending player
                let power = game.card(attacker_id).power();
                if power > 0 {
                    game.deal_damage_to_player(defending_player, power);
                }
            } else {
                // Blocked — mutual damage
                let attacker_power = game.card(attacker_id).power();
                let mut remaining_damage = attacker_power;

                for &blocker_id in &blockers {
                    let blocker_power = game.card(blocker_id).power();
                    let blocker_toughness = game.card(blocker_id).toughness();

                    // Attacker damages blocker
                    let damage_to_blocker = remaining_damage.min(blocker_toughness);
                    if damage_to_blocker > 0 {
                        game.deal_damage_to_card(blocker_id, damage_to_blocker);
                        remaining_damage -= damage_to_blocker;
                    }

                    // Blocker damages attacker
                    if blocker_power > 0 {
                        game.deal_damage_to_card(attacker_id, blocker_power);
                    }
                }

                // Trample: remaining damage goes to player (not implemented yet)
            }
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

/// Parse NumDmg$ value from an ability string like "NumDmg$ 3".
fn parse_num_dmg(ability: &str) -> i32 {
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("NumDmg$ ") {
            if let Ok(n) = val.trim().parse::<i32>() {
                return n;
            }
        }
    }
    0
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
}
