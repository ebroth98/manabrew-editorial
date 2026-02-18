use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// Tracks combat state for the current combat phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatState {
    /// Attacking player.
    pub attacking_player: Option<PlayerId>,
    /// Defending player.
    pub defending_player: Option<PlayerId>,
    /// (attacker CardId, defending player)
    pub attackers: Vec<(CardId, PlayerId)>,
    /// (blocker CardId, attacker CardId)
    pub blockers: Vec<(CardId, CardId)>,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.attacking_player = None;
        self.defending_player = None;
        self.attackers.clear();
        self.blockers.clear();
    }

    pub fn declare_attacker(&mut self, attacker: CardId, defending: PlayerId) {
        self.attackers.push((attacker, defending));
    }

    pub fn declare_blocker(&mut self, blocker: CardId, attacker: CardId) {
        self.blockers.push((blocker, attacker));
    }

    pub fn is_attacking(&self, card: CardId) -> bool {
        self.attackers.iter().any(|(a, _)| *a == card)
    }

    pub fn is_blocked(&self, attacker: CardId) -> bool {
        self.blockers.iter().any(|(_, a)| *a == attacker)
    }

    pub fn get_blockers_for(&self, attacker: CardId) -> Vec<CardId> {
        self.blockers
            .iter()
            .filter(|(_, a)| *a == attacker)
            .map(|(b, _)| *b)
            .collect()
    }

    pub fn has_attackers(&self) -> bool {
        !self.attackers.is_empty()
    }
}
