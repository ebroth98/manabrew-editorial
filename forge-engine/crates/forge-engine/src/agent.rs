use crate::ids::{CardId, PlayerId};

/// Trait for player decision-making. Decouples the engine from UI/AI.
/// Implementations can be interactive (prompt user), AI, or network-driven.
pub trait PlayerAgent {
    /// Choose which cards to keep in opening hand (mulligan decision).
    /// Returns true to keep, false to mulligan.
    fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId]) -> bool;

    /// Choose a card to play from hand during a main phase.
    /// Return None to pass priority.
    fn choose_action(&mut self, player: PlayerId, playable: &[CardId]) -> Option<CardId>;

    /// Choose attackers from available creatures.
    /// Returns the set of creature CardIds to declare as attackers.
    fn choose_attackers(&mut self, player: PlayerId, available: &[CardId]) -> Vec<CardId>;

    /// Choose blockers. Returns pairs of (blocker, attacker).
    fn choose_blockers(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)>;

    /// Choose a target player (e.g. for Lightning Bolt targeting a player).
    fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId>;

    /// Choose a target card (e.g. for Lightning Bolt targeting a creature).
    fn choose_target_card(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId>;

    /// Choose whether to play a land or cast a spell when both are possible.
    /// Returns true for land, false for spell, None to pass.
    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool>;

    /// Notify the agent of a game event (for display/logging).
    fn notify(&mut self, message: &str);
}

/// A simple agent that always passes priority and makes no choices.
/// Useful for testing.
pub struct PassAgent;

impl PlayerAgent for PassAgent {
    fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
        true // always keep
    }

    fn choose_action(&mut self, _player: PlayerId, _playable: &[CardId]) -> Option<CardId> {
        None // always pass
    }

    fn choose_attackers(&mut self, _player: PlayerId, _available: &[CardId]) -> Vec<CardId> {
        Vec::new() // no attackers
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        _attackers: &[CardId],
        _available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        Vec::new() // no blockers
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

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {}
}
