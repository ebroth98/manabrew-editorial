use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana_pool::ManaPool;

/// A target choice that can be a player, a card, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetChoice {
    Player(PlayerId),
    Card(CardId),
    None,
}

/// The action a player takes during a main phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPhaseAction {
    /// Pass priority / end main phase.
    Pass,
    /// Play a card from hand (land or spell).
    Play(CardId),
    /// Tap an untapped land on the battlefield to add its mana to the pool.
    ActivateMana(CardId),
    /// Untap a tapped land and remove its mana from the pool (undo tap).
    UntapMana(CardId),
}

/// Trait for player decision-making. Decouples the engine from UI/AI.
/// Implementations can be interactive (prompt user), AI, or network-driven.
pub trait PlayerAgent {
    /// Called before each agent decision point with the current game state.
    /// Override this to capture snapshots for a UI or network layer.
    fn snapshot_state(&mut self, _game: &GameState, _mana_pools: &[ManaPool]) {}

    /// Choose which cards to keep in opening hand (mulligan decision).
    /// Returns true to keep, false to mulligan.
    fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId]) -> bool;

    /// Choose a main-phase action: play a card from hand, tap a land for mana, untap a land, or pass.
    /// `tappable_lands` lists untapped lands available for tapping.
    /// `untappable_lands` lists tapped lands that still have mana in the pool (can be untapped).
    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
    ) -> MainPhaseAction;

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

    /// Choose a target that can be a player or a card (e.g. "any target").
    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice;

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

    fn choose_action(
        &mut self,
        _player: PlayerId,
        _playable: &[CardId],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
    ) -> MainPhaseAction {
        MainPhaseAction::Pass
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

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        if let Some(&pid) = valid_players.first() {
            TargetChoice::Player(pid)
        } else if let Some(&cid) = valid_cards.first() {
            TargetChoice::Card(cid)
        } else {
            TargetChoice::None
        }
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {}
}
