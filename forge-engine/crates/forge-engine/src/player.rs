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
        }
    }

    pub fn gain_life(&mut self, amount: i32) {
        self.life += amount;
        self.life_gained_this_turn += amount;
    }

    pub fn lose_life(&mut self, amount: i32) {
        self.life -= amount;
        self.life_lost_this_turn += amount;
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
