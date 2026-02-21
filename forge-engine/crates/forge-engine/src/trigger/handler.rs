use std::collections::HashSet;

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::{SpellAbility, StackEntry};
use crate::trigger::TriggerMode;

/// An active trigger reference — (card_id, trigger_index) pair.
/// In Java this is a direct Trigger object reference. In Rust,
/// we use IDs because of the arena pattern (no references).
#[derive(Debug, Clone)]
struct ActiveTrigger {
    card_id: CardId,
    trigger_index: usize,
}

/// Mirrors Java's TriggerWaiting.
#[derive(Debug, Clone)]
struct TriggerWaiting {
    mode: TriggerType,
    params: RunParams,
}

/// A one-shot delayed trigger.
#[derive(Debug, Clone)]
pub struct DelayedTrigger {
    pub mode: TriggerType,
    pub trigger_mode: TriggerMode,
    pub execute_svar: String,
    pub controller: PlayerId,
    pub source_card: CardId,
}

/// Mirrors Java's TriggerHandler — central trigger dispatcher.
/// In Java, lives on Game. In Rust, lives on GameLoop because
/// active_triggers and waiting_triggers are transient processing state.
#[allow(dead_code)]
pub struct TriggerHandler {
    active_triggers: Vec<ActiveTrigger>,
    waiting_triggers: Vec<TriggerWaiting>,
    delayed_triggers: Vec<DelayedTrigger>,
    suppressed_modes: HashSet<TriggerType>,
    next_trigger_id: u32,
}

impl TriggerHandler {
    pub fn new() -> Self {
        TriggerHandler {
            active_triggers: Vec::new(),
            waiting_triggers: Vec::new(),
            delayed_triggers: Vec::new(),
            suppressed_modes: HashSet::new(),
            next_trigger_id: 0,
        }
    }

    /// Mirrors Java's runTrigger() — main entry point.
    /// Called from game actions when events occur.
    /// If `hold` is true, event is queued; otherwise it's also queued
    /// (all triggers go through the waiting queue for APNAP ordering).
    pub fn run_trigger(&mut self, mode: TriggerType, params: RunParams, _hold: bool) {
        self.waiting_triggers.push(TriggerWaiting { mode, params });
    }

    /// Mirrors Java's runWaitingTriggers().
    /// Drains waiting queue, matches triggers, returns StackEntries.
    pub fn run_waiting_triggers(&mut self, game: &GameState) -> Vec<StackEntry> {
        if self.waiting_triggers.is_empty() {
            return Vec::new();
        }

        let waiting = std::mem::take(&mut self.waiting_triggers);
        let mut entries = Vec::new();

        for event in &waiting {
            // Check each active trigger
            for active in &self.active_triggers {
                let card = game.card(active.card_id);
                if active.trigger_index >= card.triggers.len() {
                    continue;
                }
                let trigger = &card.triggers[active.trigger_index];
                let host_controller = card.controller;

                if self.can_run_trigger(
                    game,
                    active.card_id,
                    active.trigger_index,
                    host_controller,
                    &event.mode,
                    &event.params,
                ) {
                    // Look up the SVar for the execute key
                    let svar_text = card
                        .svars
                        .get(&trigger.execute)
                        .cloned()
                        .unwrap_or_default();

                    let mut sa = SpellAbility::new_simple(
                        Some(active.card_id),
                        host_controller,
                        &svar_text,
                    );
                    sa.is_trigger = true;
                    sa.trigger_source = Some(active.card_id);
                    sa.trigger_index = Some(active.trigger_index);

                    let entry = StackEntry {
                        id: 0,
                        spell_ability: sa,
                        is_creature_spell: false,
                        is_permanent_spell: false,
                    };
                    entries.push((entry, host_controller));
                }
            }
        }

        // APNAP ordering: active player's triggers first
        let active_player = game.active_player();
        entries.sort_by_key(|(_, controller)| if *controller == active_player { 0 } else { 1 });

        entries.into_iter().map(|(entry, _)| entry).collect()
    }

    /// Mirrors Java's resetActiveTriggers().
    /// Scans all cards in game, collects active triggers.
    pub fn reset_active_triggers(&mut self, game: &GameState) {
        self.active_triggers.clear();
        for (i, card) in game.cards.iter().enumerate() {
            let card_id = CardId(i as u32);
            for (trig_idx, trigger) in card.triggers.iter().enumerate() {
                if trigger.active_zones.contains(&card.zone) {
                    self.active_triggers.push(ActiveTrigger {
                        card_id,
                        trigger_index: trig_idx,
                    });
                }
            }
        }
    }

    /// Mirrors Java's registerActiveTrigger(card).
    /// Registers a single card's triggers.
    pub fn register_active_trigger(&mut self, game: &GameState, card_id: CardId) {
        let card = game.card(card_id);
        for (trig_idx, trigger) in card.triggers.iter().enumerate() {
            if trigger.active_zones.contains(&card.zone) {
                // Avoid duplicates
                let already_registered = self.active_triggers.iter().any(|at| {
                    at.card_id == card_id && at.trigger_index == trig_idx
                });
                if !already_registered {
                    self.active_triggers.push(ActiveTrigger {
                        card_id,
                        trigger_index: trig_idx,
                    });
                }
            }
        }
    }

    /// Remove triggers for a card that left a trigger zone.
    pub fn unregister_active_triggers(&mut self, card_id: CardId) {
        self.active_triggers
            .retain(|at| at.card_id != card_id);
    }

    /// Mirrors Java's canRunTrigger().
    /// Validation chain: mode → suppression → active zones → performTest.
    fn can_run_trigger(
        &self,
        game: &GameState,
        host_card: CardId,
        trigger_index: usize,
        host_controller: PlayerId,
        mode: &TriggerType,
        params: &RunParams,
    ) -> bool {
        let card = game.card(host_card);
        if trigger_index >= card.triggers.len() {
            return false;
        }
        let trigger = &card.triggers[trigger_index];

        // Check mode matches trigger type
        let trigger_type = match &trigger.mode {
            TriggerMode::ChangesZone { .. } => TriggerType::ChangesZone,
            TriggerMode::Phase { .. } => TriggerType::Phase,
            TriggerMode::SpellCast { .. } => TriggerType::SpellCast,
            TriggerMode::Attacks { .. } => TriggerType::Attacks,
            TriggerMode::DamageDone { .. } => TriggerType::DamageDone,
        };

        if trigger_type != *mode {
            return false;
        }

        // Check suppression
        if self.suppressed_modes.contains(mode) {
            return false;
        }

        // Check active zones
        if !trigger.active_zones.contains(&card.zone) {
            return false;
        }

        // performTest
        trigger
            .mode
            .perform_test(params, game, host_card, host_controller)
    }

    pub fn suppress_mode(&mut self, mode: TriggerType) {
        self.suppressed_modes.insert(mode);
    }

    pub fn clear_suppression(&mut self, mode: TriggerType) {
        self.suppressed_modes.remove(&mode);
    }
}

impl Default for TriggerHandler {
    fn default() -> Self {
        Self::new()
    }
}
