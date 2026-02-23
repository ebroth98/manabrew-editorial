use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::ids::{CardId, PlayerId};

/// Simple AI that plays the first available card, attacks with everything,
/// and targets the first valid option.
pub struct SimpleAiAgent;

impl PlayerAgent for SimpleAiAgent {
    fn mulligan_decision(&mut self, _: PlayerId, _: &[CardId]) -> bool {
        true // always keep
    }

    fn choose_action(
        &mut self,
        _: PlayerId,
        playable: &[CardId],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        _activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        playable
            .first()
            .copied()
            .map(MainPhaseAction::Play)
            .unwrap_or(MainPhaseAction::Pass)
    }

    fn choose_attackers(&mut self, _: PlayerId, available: &[CardId]) -> Vec<CardId> {
        available.to_vec() // attack with everything
    }

    fn choose_blockers(
        &mut self,
        _: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        // Block the first attacker with the first blocker
        if !attackers.is_empty() && !available_blockers.is_empty() {
            vec![(available_blockers[0], attackers[0])]
        } else {
            Vec::new()
        }
    }

    fn choose_target_player(&mut self, _: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        valid.first().copied()
    }

    fn choose_target_card(&mut self, _: PlayerId, valid: &[CardId]) -> Option<CardId> {
        valid.first().copied()
    }

    fn choose_target_any(
        &mut self,
        _: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        // AI prefers targeting creatures, then players
        if let Some(&cid) = valid_cards.first() {
            TargetChoice::Card(cid)
        } else if let Some(&pid) = valid_players.first() {
            TargetChoice::Player(pid)
        } else {
            TargetChoice::None
        }
    }

    fn choose_land_or_spell(&mut self, _: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {}
}
