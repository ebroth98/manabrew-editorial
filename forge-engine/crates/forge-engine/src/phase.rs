use forge_foundation::PhaseType;
use serde::{Deserialize, Serialize};

use crate::ids::PlayerId;

/// Tracks the current turn state: whose turn, which phase, turn number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub turn_number: u32,
    pub active_player: PlayerId,
    pub phase: PhaseType,
    pub priority_player: PlayerId,
    pub num_players: u32,

    // Combat tracking
    pub combat_attackers_declared: bool,
    pub combat_blockers_declared: bool,

    // Per-turn flags
    pub drawn_for_turn: bool,
}

impl TurnState {
    pub fn new(active_player: PlayerId, num_players: u32) -> Self {
        TurnState {
            turn_number: 1,
            active_player,
            phase: PhaseType::Untap,
            priority_player: active_player,
            num_players,
            combat_attackers_declared: false,
            combat_blockers_declared: false,
            drawn_for_turn: false,
        }
    }

    /// Advance to the next phase. Returns true if the turn ended (wrapped to Untap).
    pub fn advance_phase(&mut self) -> bool {
        let next = self.phase.next();
        let turn_ended = next == PhaseType::Untap && self.phase == PhaseType::Cleanup;
        self.phase = next;

        if turn_ended {
            self.turn_number += 1;
            self.combat_attackers_declared = false;
            self.combat_blockers_declared = false;
            self.drawn_for_turn = false;
        }

        // Reset combat flags when entering combat
        if self.phase == PhaseType::CombatBegin {
            self.combat_attackers_declared = false;
            self.combat_blockers_declared = false;
        }

        turn_ended
    }

    /// Advance to the next player's turn (for multiplayer).
    pub fn next_player_turn(&mut self, player_order: &[PlayerId]) {
        if let Some(pos) = player_order
            .iter()
            .position(|&p| p == self.active_player)
        {
            let next = (pos + 1) % player_order.len();
            self.active_player = player_order[next];
            self.priority_player = self.active_player;
        }
    }

    pub fn is_main_phase(&self) -> bool {
        self.phase.is_main()
    }

    pub fn is_combat(&self) -> bool {
        self.phase.is_combat()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_phases() {
        let mut ts = TurnState::new(PlayerId(0), 2);
        assert_eq!(ts.phase, PhaseType::Untap);

        ts.advance_phase();
        assert_eq!(ts.phase, PhaseType::Upkeep);

        ts.advance_phase();
        assert_eq!(ts.phase, PhaseType::Draw);
    }

    #[test]
    fn turn_wraps() {
        let mut ts = TurnState::new(PlayerId(0), 2);
        assert_eq!(ts.turn_number, 1);

        // Advance through all phases
        loop {
            let ended = ts.advance_phase();
            if ended {
                break;
            }
        }
        assert_eq!(ts.turn_number, 2);
        assert_eq!(ts.phase, PhaseType::Untap);
    }
}
