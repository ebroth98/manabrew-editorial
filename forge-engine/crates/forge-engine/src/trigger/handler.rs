use std::collections::HashSet;

use forge_foundation::ZoneType;

use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::{build_spell_ability, StackEntry};
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
    /// Optional target card for the delayed trigger (e.g. the creature to bounce for Dash
    /// or sacrifice for Blitz at end of turn).
    pub target_card: Option<CardId>,
    /// Sum of integer values remembered by the delayed trigger at creation.
    pub remembered_amount: i32,
}

/// A triggered ability ready to be placed on the stack, with optional metadata.
#[derive(Debug, Clone)]
pub struct PendingTrigger {
    pub entry: StackEntry,
    /// Whether this trigger is optional (has OptionalDecider$).
    pub optional: bool,
    /// The player who decides whether the trigger fires (usually the controller).
    pub decider: PlayerId,
    /// Description text for the trigger (shown to player for optional triggers).
    pub description: String,
}

/// Mirrors Java's TriggerHandler — central trigger dispatcher.
/// In Java, lives on Game. In Rust, lives on GameLoop because
/// active_triggers and waiting_triggers are transient processing state.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TriggerHandler {
    active_triggers: Vec<ActiveTrigger>,
    waiting_triggers: Vec<TriggerWaiting>,
    delayed_triggers: Vec<DelayedTrigger>,
    suppressed_modes: HashSet<TriggerType>,
    next_trigger_id: u32,
    /// Triggers that were matched early (before SBA) and are waiting to be
    /// placed on the stack. This ensures triggers from creatures that die to
    /// SBA (e.g. Raptor Hatchling's enrage) are not lost.
    pre_matched_triggers: Vec<(PendingTrigger, PlayerId)>,
}

impl TriggerHandler {
    pub fn new() -> Self {
        TriggerHandler {
            active_triggers: Vec::new(),
            waiting_triggers: Vec::new(),
            delayed_triggers: Vec::new(),
            suppressed_modes: HashSet::new(),
            next_trigger_id: 0,
            pre_matched_triggers: Vec::new(),
        }
    }

    /// Mirrors Java's runTrigger() — main entry point.
    /// Called from game actions when events occur.
    /// If `hold` is true, event is queued; otherwise it's also queued
    /// (all triggers go through the waiting queue for APNAP ordering).
    pub fn run_trigger(&mut self, mode: TriggerType, params: RunParams, _hold: bool) {
        self.waiting_triggers.push(TriggerWaiting { mode, params });
    }

    /// Match waiting triggers NOW, while source cards are still in their
    /// current zones.  Stores results in `pre_matched_triggers` so that a
    /// subsequent `run_waiting_triggers` call returns them even if SBA has
    /// since moved the source card (e.g. Raptor Hatchling dying to combat
    /// damage before triggers go on the stack).
    ///
    /// Call this immediately after firing triggers and before SBA.
    pub fn flush_waiting_triggers(&mut self, game: &GameState) {
        if self.waiting_triggers.is_empty() && self.delayed_triggers.is_empty() {
            return;
        }
        let matched = self.match_waiting_triggers(game);
        self.pre_matched_triggers.extend(matched);
    }

    /// Mirrors Java's runWaitingTriggers().
    /// Drains waiting queue, matches triggers, returns PendingTriggers.
    /// The caller (game_loop) handles OptionalDecider$ prompting.
    pub fn run_waiting_triggers(&mut self, game: &GameState) -> Vec<PendingTrigger> {
        // Start with any triggers that were pre-matched (flushed before SBA).
        let mut entries: Vec<(PendingTrigger, PlayerId)> =
            std::mem::take(&mut self.pre_matched_triggers);

        if self.waiting_triggers.is_empty() && self.delayed_triggers.is_empty() {
            if entries.is_empty() {
                return Vec::new();
            }
            // Only have pre-matched — apply APNAP ordering and return.
            let active_player = game.active_player();
            entries.sort_by_key(|(_, controller)| if *controller == active_player { 0 } else { 1 });
            return entries.into_iter().map(|(pending, _)| pending).collect();
        }

        // Match any remaining waiting triggers (those fired after the flush).
        entries.extend(self.match_waiting_triggers(game));

        // APNAP ordering: active player's triggers first
        let active_player = game.active_player();
        entries.sort_by_key(|(_, controller)| if *controller == active_player { 0 } else { 1 });

        entries.into_iter().map(|(pending, _)| pending).collect()
    }

    /// Drain `waiting_triggers`, match them against active and delayed triggers,
    /// and return the matched entries.  This is the core matching logic shared by
    /// both `flush_waiting_triggers` and `run_waiting_triggers`.
    fn match_waiting_triggers(&mut self, game: &GameState) -> Vec<(PendingTrigger, PlayerId)> {
        let waiting = std::mem::take(&mut self.waiting_triggers);
        let mut entries: Vec<(PendingTrigger, PlayerId)> = Vec::new();

        for event in &waiting {
            // Check each active trigger
            for active in &self.active_triggers {
                let card = game.card(active.card_id);
                if active.trigger_index >= card.triggers.len() {
                    continue;
                }
                let trigger = &card.triggers[active.trigger_index];
                let host_controller = card.controller;
                if crate::staticability::static_ability_disable_triggers::is_disabled(
                    game,
                    active.card_id,
                    trigger,
                    &event.params,
                ) {
                    continue;
                }

                if self.can_run_trigger(
                    game,
                    active.card_id,
                    active.trigger_index,
                    host_controller,
                    &event.mode,
                    &event.params,
                ) {
                    let svar_text = card
                        .svars
                        .get(&trigger.execute)
                        .cloned()
                        .unwrap_or_default();

                    let mut sa =
                        build_spell_ability(game, active.card_id, &svar_text, host_controller);
                    sa.is_trigger = true;
                    sa.trigger_source = Some(active.card_id);
                    sa.trigger_index = Some(active.trigger_index);

                    // Propagate trigger target from event params so that
                    // Defined$ TriggeredTarget can resolve in downstream effects.
                    // For DamageDone triggers, this is the player/card dealt damage.
                    if let Some(pid) = event.params.damage_target_player {
                        sa.target_chosen.target_player = Some(pid);
                    }
                    if let Some(cid) = event.params.damage_target_card {
                        sa.target_chosen.target_card = Some(cid);
                    }

                    let entry = StackEntry {
                        id: 0,
                        spell_ability: sa,
                        is_creature_spell: false,
                        is_permanent_spell: false,
                        cast_from_zone: None,
                    };

                    let pending = PendingTrigger {
                        entry,
                        optional: trigger.optional,
                        decider: host_controller,
                        description: trigger.description.clone(),
                    };
                    entries.push((pending, host_controller));
                    let extra = crate::staticability::static_ability_panharmonicon::extra_triggers(
                        game,
                        active.card_id,
                        trigger,
                        &event.params,
                    );
                    for _ in 0..extra {
                        let mut sa2 =
                            build_spell_ability(game, active.card_id, &svar_text, host_controller);
                        sa2.is_trigger = true;
                        sa2.trigger_source = Some(active.card_id);
                        sa2.trigger_index = Some(active.trigger_index);
                        if let Some(pid) = event.params.damage_target_player {
                            sa2.target_chosen.target_player = Some(pid);
                        }
                        if let Some(cid) = event.params.damage_target_card {
                            sa2.target_chosen.target_card = Some(cid);
                        }
                        let extra_entry = StackEntry {
                            id: 0,
                            spell_ability: sa2,
                            is_creature_spell: false,
                            is_permanent_spell: false,
                            cast_from_zone: None,
                        };
                        entries.push((
                            PendingTrigger {
                                entry: extra_entry,
                                optional: trigger.optional,
                                decider: host_controller,
                                description: trigger.description.clone(),
                            },
                            host_controller,
                        ));
                    }
                }
            }

            // Check delayed triggers (one-shot, removed after firing).
            let mut fired_indices = Vec::new();
            for (idx, delayed) in self.delayed_triggers.iter().enumerate() {
                if delayed.mode != event.mode {
                    continue;
                }
                if !delayed.trigger_mode.perform_test(
                    &event.params,
                    game,
                    delayed.source_card,
                    delayed.controller,
                ) {
                    continue;
                }
                let mut sa = build_spell_ability(
                    game,
                    delayed.source_card,
                    &delayed.execute_svar,
                    delayed.controller,
                );
                sa.is_trigger = true;
                sa.trigger_source = Some(delayed.source_card);
                sa.trigger_remembered_amount = delayed.remembered_amount;

                let entry = StackEntry {
                    id: 0,
                    spell_ability: sa,
                    is_creature_spell: false,
                    is_permanent_spell: false,
                    cast_from_zone: None,
                };
                let pending = PendingTrigger {
                    entry,
                    optional: false,
                    decider: delayed.controller,
                    description: String::new(),
                };
                entries.push((pending, delayed.controller));
                fired_indices.push(idx);
            }

            // Remove fired delayed triggers (reverse order to preserve indices).
            for idx in fired_indices.into_iter().rev() {
                self.delayed_triggers.remove(idx);
            }
        }

        entries
    }

    /// Register a delayed trigger (one-shot, fires once then is removed).
    /// Mirrors Java's `TriggerHandler.registerDelayedTrigger()`.
    pub fn register_delayed_trigger(&mut self, delayed: DelayedTrigger) {
        self.delayed_triggers.push(delayed);
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
                let already_registered = self
                    .active_triggers
                    .iter()
                    .any(|at| at.card_id == card_id && at.trigger_index == trig_idx);
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
        self.active_triggers.retain(|at| at.card_id != card_id);
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
            TriggerMode::Countered { .. } => TriggerType::Countered,
            // New trigger modes (issue #19)
            TriggerMode::Blocks { .. } => TriggerType::Blocks,
            TriggerMode::AttackerBlocked { .. } => TriggerType::AttackerBlocked,
            TriggerMode::AttackerUnblocked { .. } => TriggerType::AttackerUnblocked,
            TriggerMode::LifeGained { .. } => TriggerType::LifeGained,
            TriggerMode::LifeLost { .. } => TriggerType::LifeLost,
            TriggerMode::CounterAdded { .. } => TriggerType::CounterAdded,
            TriggerMode::CounterRemoved { .. } => TriggerType::CounterRemoved,
            TriggerMode::Sacrificed { .. } => TriggerType::Sacrificed,
            TriggerMode::Drawn { .. } => TriggerType::Drawn,
            TriggerMode::Milled { .. } => TriggerType::Milled,
            TriggerMode::Taps { .. } => TriggerType::Taps,
            TriggerMode::Untaps { .. } => TriggerType::Untaps,
            TriggerMode::Transformed { .. } => TriggerType::Transformed,
            TriggerMode::Attached { .. } => TriggerType::Attached,
            TriggerMode::Unattached { .. } => TriggerType::Unattached,
            TriggerMode::LandPlayed { .. } => TriggerType::LandPlayed,
            TriggerMode::BecomesTarget { .. } => TriggerType::BecomesTarget,
            TriggerMode::TapsForMana { .. } => TriggerType::TapsForMana,
            TriggerMode::AbilityActivated { .. } => TriggerType::AbilityActivated,
            TriggerMode::Explored { .. } => TriggerType::Explored,
            TriggerMode::BecomeMonarch { .. } => TriggerType::BecomeMonarch,
            TriggerMode::DamageDealtOnce { .. } => TriggerType::DamageDealtOnce,
            TriggerMode::Destroyed { .. } => TriggerType::Destroyed,
            TriggerMode::Exiled { .. } => TriggerType::Exiled,
            TriggerMode::TokenCreated { .. } => TriggerType::TokenCreated,
            TriggerMode::SpellCopied { .. } => TriggerType::SpellCopied,
            // ── New trigger modes (issue #54) ──
            // Modes with their own unique event types:
            TriggerMode::AttackersDeclared { .. } => TriggerType::AttackersDeclared,
            TriggerMode::BlockersDeclared => TriggerType::BlockersDeclared,
            TriggerMode::ChangesController { .. } => TriggerType::ChangesController,
            TriggerMode::TurnBegin { .. } => TriggerType::TurnBegin,
            TriggerMode::Cycled { .. } => TriggerType::Cycled,
            TriggerMode::PhasedIn { .. } => TriggerType::PhasedIn,
            TriggerMode::PhasedOut { .. } => TriggerType::PhasedOut,
            TriggerMode::Always => TriggerType::Always,
            TriggerMode::Immediate => TriggerType::Immediate,
            TriggerMode::Surveil { .. } => TriggerType::Surveil,
            TriggerMode::Scry { .. } => TriggerType::Scry,
            TriggerMode::Foretell { .. } => TriggerType::Foretell,
            TriggerMode::SearchedLibrary { .. } => TriggerType::SearchedLibrary,
            TriggerMode::Shuffled { .. } => TriggerType::Shuffled,
            TriggerMode::ManaAdded { .. } => TriggerType::ManaAdded,
            // Companion modes — remap to base event types so existing fire points match:
            TriggerMode::DamageDoneOnce { .. } => TriggerType::DamageDone,
            TriggerMode::DamageAll { .. } => TriggerType::DamageDone,
            TriggerMode::ExcessDamage { .. } => TriggerType::DamageDone,
            TriggerMode::DamagePreventedOnce { .. } => TriggerType::DamageDone,
            TriggerMode::SpellCastAll { .. } => TriggerType::SpellCast,
            TriggerMode::SpellCastOnce { .. } => TriggerType::SpellCast,
            TriggerMode::SpellCastOfType { .. } => TriggerType::SpellCast,
            TriggerMode::LifeLostAll { .. } => TriggerType::LifeLost,
            TriggerMode::LifeGainedAll { .. } => TriggerType::LifeGained,
            TriggerMode::CounterAddedOnce { .. } => TriggerType::CounterAdded,
            TriggerMode::CounterRemovedOnce { .. } => TriggerType::CounterRemoved,
            TriggerMode::Exerted { .. } => TriggerType::Exerted,
            TriggerMode::DiscardedAll { .. } => TriggerType::Discarded,
            TriggerMode::SacrificedOnce { .. } => TriggerType::Sacrificed,
            TriggerMode::ChangesZoneAll { .. } => TriggerType::ChangesZone,
            TriggerMode::TapAll { .. } => TriggerType::Taps,
            TriggerMode::UntapAll { .. } => TriggerType::Untaps,
            TriggerMode::BecomesTargetOnce { .. } => TriggerType::BecomesTarget,
            TriggerMode::TokenCreatedOnce { .. } => TriggerType::TokenCreated,
            TriggerMode::AttackerBlockedOnce { .. } => TriggerType::AttackerBlocked,
            TriggerMode::AttackerBlockedByCreature { .. } => TriggerType::AttackerBlocked,
            TriggerMode::AttackerUnblockedOnce { .. } => TriggerType::AttackerUnblocked,
            TriggerMode::ManaExpend { .. } => TriggerType::ManaExpend,
        };

        if trigger_type != *mode {
            return false;
        }

        // Check suppression
        if self.suppressed_modes.contains(mode) {
            return false;
        }

        // Check active zones.
        // For self zone-change events, use origin as LKI zone so "dies" triggers
        // (active on Battlefield) still fire after the card has moved.
        let zone_for_active_check = if *mode == TriggerType::ChangesZone
            && params.card == Some(host_card)
            && params.origin == Some(ZoneType::Battlefield)
            && params.destination != Some(ZoneType::Battlefield)
        {
            // LKI active-zone check for "leaves battlefield" self triggers (e.g. dies).
            ZoneType::Battlefield
        } else if *mode == TriggerType::DamageDone
            && params.damage_target_card == Some(host_card)
            && trigger.active_zones.contains(&ZoneType::Battlefield)
            && card.zone != ZoneType::Battlefield
        {
            // LKI for DamageDone triggers targeting self (e.g. Raptor Hatchling
            // enrage). When combat damage kills the creature, SBAs move it to
            // graveyard before triggers are processed. The trigger was queued
            // while the card was on the battlefield, so we use LKI.
            ZoneType::Battlefield
        } else {
            card.zone
        };
        if !trigger.active_zones.contains(&zone_for_active_check) {
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
