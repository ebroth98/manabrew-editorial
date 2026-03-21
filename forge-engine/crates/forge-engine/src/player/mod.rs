use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::PlayerId;

/// Mutable game-state for a single player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub name: String,

    // Life
    pub life: i32,
    pub starting_life: i32,
    pub life_gained_this_turn: i32,
    pub life_lost_this_turn: i32,

    // Poison
    pub poison_counters: i32,

    // Resources
    pub lands_played_this_turn: i32,
    pub max_land_plays_per_turn: i32,
    pub spells_cast_this_turn: i32,

    // Hand size
    pub max_hand_size: i32,

    // Draw tracking
    pub drawn_this_turn: i32,

    // Game status
    pub has_lost: bool,
    pub has_won: bool,
    pub has_conceded: bool,

    // Commander damage received: card_id.0 → total damage dealt by that commander
    pub commander_damage_received: HashMap<u32, i32>,

    // Skip turns counter (issue #22, SkipTurn effect).
    pub skip_turns: i32,

    // Phase skip flags (issue #22, SkipPhase effect).
    pub skip_next_draw: bool,
    pub skip_next_combat: bool,
    pub skip_next_untap: bool,

    // Damage prevention shields (issue #53, PreventDamage effect). Resets at EOT.
    pub damage_prevention: i32,

    // Energy counters (Kaladesh block). Persistent resource like mana.
    pub energy_counters: i32,
    // Adventure shard resource used by PayShards costs.
    pub mana_shards: i32,

    // Mana expend tracking: cumulative mana spent on spells this turn (for Expend triggers).
    pub mana_expended_this_turn: i32,

    /// Mindslaver effect: another player controls this player's decisions.
    pub controlled_by: Option<PlayerId>,

    /// City's Blessing (Ascend). Once gained, lasts for the rest of the game.
    pub has_city_blessing: bool,
    /// The Ring tempts you — current ring level (0-4).
    pub ring_level: i32,
    /// The Ring-bearer creature (if any).
    pub ring_bearer: Option<crate::ids::CardId>,
    /// Radiation counters (Fallout set mechanic).
    pub radiation_counters: i32,
}

impl PlayerState {
    pub fn new(id: PlayerId, name: String, starting_life: i32) -> Self {
        PlayerState {
            id,
            name,
            life: starting_life,
            starting_life,
            life_gained_this_turn: 0,
            life_lost_this_turn: 0,
            poison_counters: 0,
            lands_played_this_turn: 0,
            max_land_plays_per_turn: 1,
            spells_cast_this_turn: 0,
            max_hand_size: 7,
            drawn_this_turn: 0,
            has_lost: false,
            has_won: false,
            has_conceded: false,
            commander_damage_received: HashMap::new(),
            skip_turns: 0,
            skip_next_draw: false,
            skip_next_combat: false,
            skip_next_untap: false,
            damage_prevention: 0,
            energy_counters: 0,
            mana_shards: 0,
            mana_expended_this_turn: 0,
            controlled_by: None,
            has_city_blessing: false,
            ring_level: 0,
            ring_bearer: None,
            radiation_counters: 0,
        }
    }

    pub fn gain_life(&mut self, amount: i32) {
        if std::env::var("FORGE_LIFE_TRACE").is_ok() {
            eprintln!("[LIFE_TRACE] P{} gain_life({}) => life {} -> {}", self.id.index(), amount, self.life, self.life + amount);
        }
        self.life += amount;
        self.life_gained_this_turn += amount;
    }

    pub fn lose_life(&mut self, amount: i32) {
        if std::env::var("FORGE_LIFE_TRACE").is_ok() {
            eprintln!("[LIFE_TRACE] P{} lose_life({}) => life {} -> {}", self.id.index(), amount, self.life, self.life - amount);
            if self.id.index() == 1 && amount == 1 {
                eprintln!("[LIFE_TRACE] P1 1-damage backtrace:\n{}", std::backtrace::Backtrace::force_capture());
            }
        }
        self.life -= amount;
        self.life_lost_this_turn += amount;
    }

    /// Set life total to an absolute value. Returns the difference (positive = gained, negative = lost).
    /// Mirrors Java's `Player.setLife()` used by LifeSetEffect.
    pub fn set_life(&mut self, amount: i32) -> i32 {
        let diff = amount - self.life;
        self.life = amount;
        if diff > 0 {
            self.life_gained_this_turn += diff;
        } else if diff < 0 {
            self.life_lost_this_turn += diff.abs();
        }
        diff
    }

    pub fn deal_damage(&mut self, amount: i32) {
        if amount > 0 {
            self.lose_life(amount);
        }
    }

    pub fn can_play_land(&self) -> bool {
        self.lands_played_this_turn < self.max_land_plays_per_turn
    }

    pub fn is_alive(&self) -> bool {
        !self.has_lost && !self.has_conceded
    }

    /// Reset per-turn counters.
    pub fn new_turn(&mut self) {
        self.lands_played_this_turn = 0;
        self.spells_cast_this_turn = 0;
        self.life_gained_this_turn = 0;
        self.life_lost_this_turn = 0;
        self.drawn_this_turn = 0;
        self.mana_expended_this_turn = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_life() {
        let mut p = PlayerState::new(PlayerId(0), "Alice".to_string(), 20);
        assert_eq!(p.life, 20);
        p.deal_damage(3);
        assert_eq!(p.life, 17);
        p.gain_life(2);
        assert_eq!(p.life, 19);
    }

    #[test]
    fn land_plays() {
        let mut p = PlayerState::new(PlayerId(0), "Alice".to_string(), 20);
        assert!(p.can_play_land());
        p.lands_played_this_turn = 1;
        assert!(!p.can_play_land());
    }
}
