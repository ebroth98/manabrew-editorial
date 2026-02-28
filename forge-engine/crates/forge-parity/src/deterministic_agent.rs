//! A fully deterministic [`PlayerAgent`] for reproducible game traces.
//!
//! Given the same options, always returns the same choice. Uses card names for
//! ordering (not internal IDs) so the same logic can be replicated in the Java
//! harness.
//!
//! Strategy:
//! - Always keep opening hand
//! - Play first playable card (alphabetical by name), then pass
//! - Attack with all eligible creatures (sorted by name)
//! - Don't block
//! - Target opponent for player targets; first alphabetical card for card targets
//! - Discard first N alphabetically
//! - Scry: keep all on top; Surveil: mill none
//! - Pick first N modes; always accept optional triggers

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_foundation::PhaseType;

/// A deterministic agent that makes reproducible choices for parity testing.
pub struct DeterministicAgent {
    /// The player this agent controls.
    player_id: PlayerId,
    /// Log of notification messages (for debugging).
    pub log: Vec<String>,
    /// If true, print decisions to stderr.
    pub verbose: bool,
    /// Cached game state reference for name lookups.
    last_game_snapshot: Option<GameSnapshot>,
    /// Current phase — used to only play spells during main phases.
    current_phase: Option<PhaseType>,
}

/// Minimal cached state for looking up card names and types from IDs.
struct GameSnapshot {
    card_names: Vec<(CardId, String)>,
    card_is_land: Vec<(CardId, bool)>,
}

impl DeterministicAgent {
    pub fn new(player_id: PlayerId, verbose: bool) -> Self {
        Self {
            player_id,
            log: Vec::new(),
            verbose,
            last_game_snapshot: None,
            current_phase: None,
        }
    }

    /// Look up a card name from the cached snapshot.
    fn card_name(&self, id: CardId) -> String {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, name) in &snap.card_names {
                if *cid == id {
                    return name.clone();
                }
            }
        }
        format!("Card({})", id.0)
    }

    /// Check if a card is a land from the cached snapshot.
    fn is_land(&self, id: CardId) -> bool {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, land) in &snap.card_is_land {
                if *cid == id {
                    return *land;
                }
            }
        }
        false
    }

    /// Sort card IDs alphabetically by their name.
    fn sort_by_name(&self, ids: &[CardId]) -> Vec<CardId> {
        let mut sorted: Vec<(CardId, String)> =
            ids.iter().map(|&id| (id, self.card_name(id))).collect();
        sorted.sort_by(|a, b| a.1.cmp(&b.1));
        sorted.into_iter().map(|(id, _)| id).collect()
    }

    fn log_decision(&self, msg: &str) {
        if self.verbose {
            eprintln!("[parity-agent p{}] {}", self.player_id.0, msg);
        }
    }
}

impl PlayerAgent for DeterministicAgent {
    fn snapshot_state(&mut self, game: &GameState, _mana_pools: &[ManaPool]) {
        // Cache card names and land status for deterministic ordering
        let card_names: Vec<(CardId, String)> = game
            .cards
            .iter()
            .map(|c| (c.id, c.card_name.clone()))
            .collect();
        let card_is_land: Vec<(CardId, bool)> =
            game.cards.iter().map(|c| (c.id, c.is_land())).collect();
        self.last_game_snapshot = Some(GameSnapshot {
            card_names,
            card_is_land,
        });
    }

    fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
        self.log_decision("Mulligan: KEEP");
        true // always keep
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[CardId],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        _activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        // Only play spells during main phases — matches Java's AI behavior which
        // passes during non-main-phase priority windows (upkeep, combat, etc.).
        let is_main_phase = matches!(
            self.current_phase,
            Some(PhaseType::Main1) | Some(PhaseType::Main2)
        );
        if !is_main_phase || playable.is_empty() {
            self.log_decision("Main phase: PASS (nothing playable)");
            return MainPhaseAction::Pass;
        }

        // Partition playable into lands and spells, sort each alphabetically.
        // Play lands first (matching Java's DeterministicController which
        // explicitly checks land plays before spell plays).
        let lands: Vec<CardId> = playable
            .iter()
            .copied()
            .filter(|&id| self.is_land(id))
            .collect();
        let spells: Vec<CardId> = playable
            .iter()
            .copied()
            .filter(|&id| !self.is_land(id))
            .collect();

        let sorted_lands = self.sort_by_name(&lands);
        let sorted_spells = self.sort_by_name(&spells);

        let first = if let Some(&land) = sorted_lands.first() {
            land
        } else if let Some(&spell) = sorted_spells.first() {
            spell
        } else {
            self.log_decision("Main phase: PASS (nothing playable)");
            return MainPhaseAction::Pass;
        };

        let first_name = self.card_name(first);
        self.log_decision(&format!("Main phase: PLAY {}", first_name));
        MainPhaseAction::Play(first)
    }

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        // Attack with all eligible creatures, sorted by name
        let sorted = self.sort_by_name(available);
        if self.verbose && !sorted.is_empty() {
            let names: Vec<String> = sorted.iter().map(|&id| self.card_name(id)).collect();
            self.log_decision(&format!("Attackers: {}", names.join(", ")));
        }
        sorted
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        _attackers: &[CardId],
        _available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        // Don't block
        self.log_decision("Blockers: NONE");
        Vec::new()
    }

    fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        // Target opponent if possible, otherwise first valid
        let target = valid
            .iter()
            .find(|&&p| p != player)
            .or_else(|| valid.first())
            .copied();
        if let Some(t) = target {
            self.log_decision(&format!("Target player: P{}", t.0));
        }
        target
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        // First alphabetically
        let sorted = self.sort_by_name(valid);
        let target = sorted.first().copied();
        if let Some(t) = target {
            self.log_decision(&format!("Target card: {}", self.card_name(t)));
        }
        target
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        _zone: forge_foundation::ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        self.choose_target_card(player, valid)
    }

    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        // Prefer targeting opponent
        if let Some(&pid) = valid_players.iter().find(|&&p| p != player) {
            self.log_decision(&format!("Target any: Player P{}", pid.0));
            return TargetChoice::Player(pid);
        }
        if let Some(&pid) = valid_players.first() {
            self.log_decision(&format!("Target any: Player P{}", pid.0));
            return TargetChoice::Player(pid);
        }
        // First card alphabetically
        let sorted = self.sort_by_name(valid_cards);
        if let Some(&cid) = sorted.first() {
            self.log_decision(&format!("Target any: Card {}", self.card_name(cid)));
            return TargetChoice::Card(cid);
        }
        self.log_decision("Target any: NONE");
        TargetChoice::None
    }

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        // First alphabetically
        let sorted = self.sort_by_name(valid);
        let target = sorted.first().copied();
        if let Some(t) = target {
            self.log_decision(&format!("Sacrifice: {}", self.card_name(t)));
        }
        target
    }

    fn choose_scry(&mut self, _player: PlayerId, _cards: &[CardId]) -> Vec<CardId> {
        // Keep all on top
        self.log_decision("Scry: keep all on top");
        vec![]
    }

    fn choose_surveil(&mut self, _player: PlayerId, _cards: &[CardId]) -> Vec<CardId> {
        // Mill none
        self.log_decision("Surveil: mill none");
        vec![]
    }

    fn choose_dig(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        _optional: bool,
    ) -> Vec<CardId> {
        // Take first `max` alphabetically
        let sorted = self.sort_by_name(valid);
        sorted.into_iter().take(max).collect()
    }

    fn choose_reorder_library(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        // Keep original order
        cards.to_vec()
    }

    fn choose_may_shuffle(&mut self, _player: PlayerId) -> bool {
        false
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        // Discard first N alphabetically
        let sorted = self.sort_by_name(hand);
        let discarded: Vec<CardId> = sorted.into_iter().take(num).collect();
        if self.verbose && !discarded.is_empty() {
            let names: Vec<String> = discarded.iter().map(|&id| self.card_name(id)).collect();
            self.log_decision(&format!("Discard: {}", names.join(", ")));
        }
        discarded
    }

    fn choose_target_spell(&mut self, _player: PlayerId, valid: &[u32]) -> Option<u32> {
        valid.first().copied()
    }

    fn choose_mode(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
        min: usize,
        _max: usize,
        _card_name: Option<&str>,
    ) -> Vec<usize> {
        // Pick first N modes
        let count = min.min(descriptions.len());
        self.log_decision(&format!("Mode: pick first {}", count));
        (0..count).collect()
    }

    fn choose_optional_trigger(
        &mut self,
        _player: PlayerId,
        description: &str,
        _card_name: Option<&str>,
    ) -> bool {
        self.log_decision(&format!("Optional trigger '{}': ACCEPT", description));
        true
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        // Prefer land
        Some(true)
    }

    fn notify(&mut self, message: &str) {
        self.log.push(message.to_string());
        if self.verbose {
            eprintln!("[parity-agent p{}] notify: {}", self.player_id.0, message);
        }
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        if self.verbose {
            eprintln!(
                "[parity-agent p{}] === Turn {} (P{} active) ===",
                self.player_id.0, turn_number, active_player.0
            );
        }
    }

    fn notify_phase_changed(&mut self, phase: PhaseType) {
        self.current_phase = Some(phase);
        if self.verbose {
            eprintln!(
                "[parity-agent p{}] --- Phase: {:?} ---",
                self.player_id.0, phase
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_keeps_hand() {
        let mut agent = DeterministicAgent::new(PlayerId(0), false);
        assert!(agent.mulligan_decision(PlayerId(0), &[]));
    }

    #[test]
    fn no_blockers() {
        let mut agent = DeterministicAgent::new(PlayerId(0), false);
        let blocks = agent.choose_blockers(PlayerId(0), &[CardId(1)], &[CardId(2)]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn prefers_opponent_target() {
        let mut agent = DeterministicAgent::new(PlayerId(0), false);
        let target = agent.choose_target_player(PlayerId(0), &[PlayerId(0), PlayerId(1)]);
        assert_eq!(target, Some(PlayerId(1)));
    }
}
