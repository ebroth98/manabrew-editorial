/// Tracks damage-related history for a card across the game.
///
/// Mirrors Java Forge's `CardDamageHistory` class, which records per-combat
/// and per-turn damage events for use by triggered abilities and combat logic.
#[derive(Debug, Clone, Default)]
pub struct DamageHistory {
    /// Total number of times this creature has attacked across the entire game.
    pub attacks_this_game: i32,
    /// If this creature attacked this combat: 1 + number of other attackers, else 0.
    pub creature_attacked_this_combat: i32,
    /// Whether this creature blocked during the current combat.
    pub creature_blocked_this_combat: bool,
    /// Whether this creature was blocked during the current combat.
    pub creature_got_blocked_this_combat: bool,
    /// Damage dealt by this creature this turn: (amount, is_combat).
    pub damage_done_this_turn: Vec<(i32, bool)>,
}

impl DamageHistory {
    /// Record that this creature is attacking alongside `num_others` other attackers.
    pub fn record_attack(&mut self, num_others: i32) {
        self.attacks_this_game += 1;
        self.creature_attacked_this_combat = 1 + num_others;
    }

    /// Record that this creature is blocking.
    pub fn record_block(&mut self) {
        self.creature_blocked_this_combat = true;
    }

    /// Record that this creature (as an attacker) was blocked.
    pub fn record_got_blocked(&mut self) {
        self.creature_got_blocked_this_combat = true;
    }

    /// Record damage dealt by this creature.
    pub fn record_damage(&mut self, amount: i32, is_combat: bool) {
        self.damage_done_this_turn.push((amount, is_combat));
    }

    /// Reset per-combat fields at end of combat.
    pub fn end_combat(&mut self) {
        self.creature_attacked_this_combat = 0;
        self.creature_blocked_this_combat = false;
        self.creature_got_blocked_this_combat = false;
    }

    /// Reset per-turn fields at start of a new turn.
    pub fn new_turn(&mut self) {
        self.damage_done_this_turn.clear();
    }
}
