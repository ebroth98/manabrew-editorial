use std::collections::HashSet;

use forge_foundation::ZoneType;

use crate::card::valid_filter;
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;
use crate::spellability::{build_spell_ability, StackEntry};
use crate::trigger::{parse_trigger, Trigger, TriggerMode};

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
    trigger_refs: Option<Vec<(CardId, usize)>>,
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
    /// Cards remembered by the delayed trigger (e.g. `RememberObjects$ Remembered`
    /// copies the source card's remembered_cards snapshot at registration time).
    /// Exposed to the executed ability via `SpellAbility::trigger_remembered`.
    #[allow(dead_code)]
    pub remembered_cards: Vec<CardId>,
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
    this_turn_delayed_triggers: Vec<DelayedTrigger>,
    player_defined_delayed_triggers: Vec<(PlayerId, DelayedTrigger)>,
    suppressed_modes: HashSet<TriggerType>,
    all_suppressed: bool,
    next_trigger_id: u32,
    /// Triggers that were matched early (before SBA) and are waiting to be
    /// placed on the stack. This ensures triggers from creatures that die to
    /// SBA (e.g. Raptor Hatchling's enrage) are not lost.
    /// Tuple: (PendingTrigger, controller, zone_timestamp of source card).
    pre_matched_triggers: Vec<(PendingTrigger, PlayerId, u64)>,
}

impl TriggerHandler {
    pub fn new() -> Self {
        TriggerHandler {
            active_triggers: Vec::new(),
            waiting_triggers: Vec::new(),
            delayed_triggers: Vec::new(),
            this_turn_delayed_triggers: Vec::new(),
            player_defined_delayed_triggers: Vec::new(),
            suppressed_modes: HashSet::new(),
            all_suppressed: false,
            next_trigger_id: 0,
            pre_matched_triggers: Vec::new(),
        }
    }

    /// Mirrors Java's runTrigger() — main entry point.
    /// Called from game actions when events occur.
    /// If `hold` is true, event is queued; otherwise it's also queued
    /// (all triggers go through the waiting queue for APNAP ordering).
    pub fn run_trigger(&mut self, mode: TriggerType, params: RunParams, hold: bool) {
        if self.is_trigger_suppressed(mode) {
            return;
        }

        // Java parity: mana triggers are not held/frozen. We don't run them inline
        // here because matching still flows through waiting processing, but we do
        // force front-of-queue delivery when hold is false.
        let urgent = !hold
            && matches!(
                mode,
                TriggerType::Always | TriggerType::TapsForMana | TriggerType::ManaAdded
            );
        if urgent {
            self.waiting_triggers
                .insert(0, self.build_waiting_trigger(mode, params.clone()));
        } else {
            self.waiting_triggers
                .push(self.build_waiting_trigger(mode, params.clone()));
        }

        if mode == TriggerType::SpellCast {
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellAbilityCast, params.clone()));
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellCastOrCopy, params.clone()));
        }
        if mode == TriggerType::AbilityCast {
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellAbilityCast, params.clone()));
        }
        if mode == TriggerType::SpellCopied {
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellCopy, params.clone()));
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellAbilityCopy, params.clone()));
            self.waiting_triggers
                .push(self.build_waiting_trigger(TriggerType::SpellCastOrCopy, params.clone()));
        }
    }

    /// Java-parity entrypoint that can resolve triggers immediately when the
    /// event should not be held/frozen.
    pub fn run_trigger_with_game(
        &mut self,
        game: &GameState,
        mode: TriggerType,
        params: RunParams,
        hold: bool,
    ) -> Vec<PendingTrigger> {
        self.run_trigger(mode, params, hold);
        let resolve_now = mode == TriggerType::Always
            || (!hold
                && !game.stack.is_frozen()
                && !matches!(mode, TriggerType::TapsForMana | TriggerType::ManaAdded));
        if resolve_now {
            self.run_waiting_triggers(game)
        } else {
            Vec::new()
        }
    }

    /// Java parity wrapper for TriggerHandler.parseTrigger(String).
    pub fn parse_trigger(&mut self, raw: &str) -> Option<Trigger> {
        parse_trigger(raw, &mut self.next_trigger_id)
    }

    /// Java parity wrapper for TriggerHandler.collectTriggerForWaiting(...).
    pub fn collect_trigger_for_waiting(&mut self, mode: TriggerType, params: RunParams) {
        self.waiting_triggers
            .push(self.build_waiting_trigger(mode, params));
    }

    /// Number of triggers in the waiting queue (for debug/diagnostics).
    pub fn waiting_trigger_count(&self) -> usize {
        self.waiting_triggers.len()
    }

    /// Match waiting triggers NOW, while source cards are still in their
    /// current zones.  Stores results in `pre_matched_triggers` so that a
    /// subsequent `run_waiting_triggers` call returns them even if SBA has
    /// since moved the source card (e.g. Raptor Hatchling dying to combat
    /// damage before triggers go on the stack).
    ///
    /// Call this immediately after firing triggers and before SBA.
    /// Returns true if any active trigger is a Drawn trigger with Number$ set.
    /// Used to gate flush_waiting_triggers during draws to avoid disrupting
    /// other trigger matching when no Number$-gated Drawn triggers exist.
    pub fn has_number_drawn_triggers(&self, game: &GameState) -> bool {
        self.active_triggers.iter().any(|at| {
            let card = game.card(at.card_id);
            if at.trigger_index >= card.triggers.len() {
                return false;
            }
            matches!(
                &card.triggers[at.trigger_index].mode,
                crate::trigger::TriggerMode::Drawn {
                    number: Some(_),
                    ..
                }
            )
        })
    }

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
        let mut entries: Vec<(PendingTrigger, PlayerId, u64)> =
            std::mem::take(&mut self.pre_matched_triggers);

        if self.waiting_triggers.is_empty() && self.delayed_triggers.is_empty() {
            if entries.is_empty() {
                return Vec::new();
            }
            // Only have pre-matched — apply APNAP + zone timestamp ordering.
            let active_player = game.active_player();
            entries.sort_by_key(|(_, controller, ts)| {
                (if *controller == active_player { 0u8 } else { 1 }, *ts)
            });
            return entries.into_iter().map(|(pending, _, _)| pending).collect();
        }

        // Match any remaining waiting triggers (those fired after the flush).
        entries.extend(self.match_waiting_triggers(game));

        // Fire Immediate delayed triggers — these fire "as soon as possible"
        // without waiting for a matching event (mirrors Java registerDelayedTrigger
        // with TriggerType.Immediate).
        entries.extend(self.fire_immediate_delayed_triggers(game));

        // APNAP ordering: active player's triggers first.
        // Within the same player, order by zone_timestamp (older cards first),
        // matching Java's forEachCardInGame() which iterates by Zone.cardList
        // insertion order.
        let active_player = game.active_player();
        entries.sort_by_key(|(_, controller, ts)| {
            (if *controller == active_player { 0u8 } else { 1 }, *ts)
        });

        entries.into_iter().map(|(pending, _, _)| pending).collect()
    }

    /// Drain `waiting_triggers`, match them against active and delayed triggers,
    /// and return the matched entries.  This is the core matching logic shared by
    /// both `flush_waiting_triggers` and `run_waiting_triggers`.
    fn match_waiting_triggers(&mut self, game: &GameState) -> Vec<(PendingTrigger, PlayerId, u64)> {
        let waiting = std::mem::take(&mut self.waiting_triggers);
        let mut entries: Vec<(PendingTrigger, PlayerId, u64)> = Vec::new();

        for event in &waiting {
            let mut trigger_refs: Vec<(CardId, usize, usize)> =
                if let Some(stored_refs) = &event.trigger_refs {
                    stored_refs
                        .iter()
                        .enumerate()
                        .map(|(idx, (card_id, trigger_index))| (*card_id, *trigger_index, idx))
                        .collect()
                } else {
                    self.active_triggers
                        .iter()
                        .enumerate()
                        .map(|(idx, active)| (active.card_id, active.trigger_index, idx))
                        .collect()
                };
            if event.trigger_refs.is_none() {
                for (card_id, trigger_index) in self.ltb_trigger_refs_for_event(game, event) {
                    if !trigger_refs.iter().any(|(existing_card, existing_idx, _)| {
                        *existing_card == card_id && *existing_idx == trigger_index
                    }) {
                        trigger_refs.push((card_id, trigger_index, usize::MAX));
                    }
                }
            }
            trigger_refs.sort_by_key(|&(card_id, trigger_index, idx)| {
                let card = game.card(card_id);
                let is_static = card
                    .triggers
                    .get(trigger_index)
                    .map(|trigger| !trigger.is_static())
                    .unwrap_or(true);
                (is_static, idx)
            });

            // Check each active trigger plus any Java-style LTB look-back triggers.
            for (card_id, trigger_index, _) in trigger_refs {
                let card = game.card(card_id);
                if trigger_index >= card.triggers.len() {
                    continue;
                }
                let trigger = &card.triggers[trigger_index];
                let host_controller = card.controller;
                if crate::staticability::static_ability_disable_triggers::is_disabled(
                    game,
                    card_id,
                    trigger,
                    &event.params,
                ) {
                    continue;
                }

                let can_run = self.can_run_trigger(
                    game,
                    card_id,
                    trigger_index,
                    host_controller,
                    &event.mode,
                    &event.params,
                );
                if can_run {
                    let sa = trigger.build_triggered_spell_ability(
                        game,
                        card_id,
                        host_controller,
                        trigger_index,
                        &event.params,
                    );

                    let entry = StackEntry {
                        id: 0,
                        spell_ability: sa,
                        is_creature_spell: false,
                        is_permanent_spell: false,
                        cast_from_zone: None,
                        optional_trigger_decider: None,
                        optional_trigger_description: None,
                        optional_trigger_source_name: None,
                    };
                    // A trigger is optional if it has OptionalDecider$ OR if its
                    // execute SVar has a non-mandatory, non-zero cost.  Mirrors Java's
                    // Trigger.isOptional() which checks both the trigger flag and cost.
                    // E.g. Smuggler's Copter loot "Cost$ Draw<1/You>" → optional.
                    let trigger_cost_optional = entry
                        .spell_ability
                        .pay_costs
                        .as_ref()
                        .map(|c| !c.mandatory && !c.is_zero_cost())
                        .unwrap_or(false);
                    let pending = PendingTrigger {
                        entry,
                        optional: trigger.optional || trigger_cost_optional,
                        decider: host_controller,
                        description: trigger.description.clone(),
                    };
                    let source_ts = card.zone_timestamp;
                    entries.push((pending, host_controller, source_ts));
                    let extra = crate::staticability::static_ability_panharmonicon::extra_triggers(
                        game,
                        card_id,
                        trigger,
                        &event.params,
                    );
                    for _ in 0..extra {
                        let sa2 = trigger.build_triggered_spell_ability(
                            game,
                            card_id,
                            host_controller,
                            trigger_index,
                            &event.params,
                        );
                        let extra_entry = StackEntry {
                            id: 0,
                            spell_ability: sa2,
                            is_creature_spell: false,
                            is_permanent_spell: false,
                            cast_from_zone: None,
                            optional_trigger_decider: None,
                            optional_trigger_description: None,
                            optional_trigger_source_name: None,
                        };
                        let trigger_cost_optional = extra_entry
                            .spell_ability
                            .pay_costs
                            .as_ref()
                            .map(|c| !c.mandatory && !c.is_zero_cost())
                            .unwrap_or(false);
                        entries.push((
                            PendingTrigger {
                                entry: extra_entry,
                                optional: trigger.optional || trigger_cost_optional,
                                decider: host_controller,
                                description: trigger.description.clone(),
                            },
                            host_controller,
                            source_ts,
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
                    None,
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
                sa.trigger_source_zone_timestamp =
                    Some(game.card(delayed.source_card).zone_timestamp);
                sa.trigger_remembered_amount = delayed.remembered_amount;
                // Propagate remembered cards (e.g. `RememberObjects$ Remembered`
                // captured at registration) so the executed ability can target
                // exactly the cards the parent trigger remembered.
                sa.trigger_remembered.extend(
                    delayed
                        .remembered_cards
                        .iter()
                        .copied()
                        .map(crate::event::AbilityValue::Card),
                );

                let entry = StackEntry {
                    id: 0,
                    spell_ability: sa,
                    is_creature_spell: false,
                    is_permanent_spell: false,
                    cast_from_zone: None,
                    optional_trigger_decider: None,
                    optional_trigger_description: None,
                    optional_trigger_source_name: None,
                };
                let pending = PendingTrigger {
                    entry,
                    optional: false,
                    decider: delayed.controller,
                    description: String::new(),
                };
                let delayed_ts = game.card(delayed.source_card).zone_timestamp;
                entries.push((pending, delayed.controller, delayed_ts));
                fired_indices.push(idx);
            }

            // Remove fired delayed triggers (reverse order to preserve indices).
            for idx in fired_indices.into_iter().rev() {
                self.delayed_triggers.remove(idx);
            }
        }

        entries
    }

    fn build_waiting_trigger(&self, mode: TriggerType, params: RunParams) -> TriggerWaiting {
        let trigger_refs = if mode == TriggerType::Drawn {
            Some(
                self.active_triggers
                    .iter()
                    .filter_map(|active| {
                        let trigger = (active.card_id, active.trigger_index);
                        Some(trigger)
                    })
                    .collect(),
            )
        } else {
            None
        };

        TriggerWaiting {
            mode,
            params,
            trigger_refs,
        }
    }

    /// Fire Immediate delayed triggers — these fire on the next trigger
    /// processing cycle without needing a matching event. Mirrors Java's
    /// `TriggerImmediate` which has `performTest()` returning true always.
    fn fire_immediate_delayed_triggers(
        &mut self,
        game: &GameState,
    ) -> Vec<(PendingTrigger, PlayerId, u64)> {
        let mut entries = Vec::new();
        let mut fired_indices = Vec::new();
        for (idx, delayed) in self.delayed_triggers.iter().enumerate() {
            if delayed.mode != TriggerType::Immediate {
                continue;
            }
            // Look up the Execute SVar on the source card
            let svar_text = game
                .card(delayed.source_card)
                .svars
                .get(&delayed.execute_svar)
                .cloned();
            if let Some(text) = svar_text {
                let mut sa =
                    build_spell_ability(game, delayed.source_card, &text, delayed.controller);
                sa.is_trigger = true;
                sa.trigger_source = Some(delayed.source_card);
                sa.trigger_source_zone_timestamp =
                    Some(game.card(delayed.source_card).zone_timestamp);

                let entry = StackEntry {
                    id: 0,
                    spell_ability: sa,
                    is_creature_spell: false,
                    is_permanent_spell: false,
                    cast_from_zone: None,
                    optional_trigger_decider: None,
                    optional_trigger_description: None,
                    optional_trigger_source_name: None,
                };
                let pending = PendingTrigger {
                    entry,
                    optional: false,
                    decider: delayed.controller,
                    description: String::new(),
                };
                let ts = game.card(delayed.source_card).zone_timestamp;
                entries.push((pending, delayed.controller, ts));
                fired_indices.push(idx);
            }
        }
        for idx in fired_indices.into_iter().rev() {
            self.delayed_triggers.remove(idx);
        }
        entries
    }

    /// Register a delayed trigger (one-shot, fires once then is removed).
    /// Mirrors Java's `TriggerHandler.registerDelayedTrigger()`.
    pub fn register_delayed_trigger(&mut self, delayed: DelayedTrigger) {
        self.delayed_triggers.push(delayed);
    }

    pub fn clear_delayed_trigger(&mut self) {
        self.delayed_triggers.clear();
        self.this_turn_delayed_triggers.clear();
    }

    pub fn register_this_turn_delayed_trigger(&mut self, delayed: DelayedTrigger) {
        self.this_turn_delayed_triggers.push(delayed.clone());
        self.delayed_triggers.push(delayed);
    }

    pub fn clear_this_turn_delayed_trigger(&mut self) {
        if self.this_turn_delayed_triggers.is_empty() {
            return;
        }
        self.delayed_triggers.retain(|d| {
            !self.this_turn_delayed_triggers.iter().any(|x| {
                x.source_card == d.source_card
                    && x.controller == d.controller
                    && x.mode == d.mode
                    && x.execute_svar == d.execute_svar
            })
        });
        self.this_turn_delayed_triggers.clear();
    }

    pub fn register_player_defined_delayed_trigger(
        &mut self,
        player: PlayerId,
        delayed: DelayedTrigger,
    ) {
        self.player_defined_delayed_triggers.push((player, delayed));
    }

    pub fn clear_player_defined_delayed_trigger(&mut self) {
        self.player_defined_delayed_triggers.clear();
    }

    pub fn handle_player_defined_del_triggers(&mut self, player: PlayerId) {
        let mut to_activate = Vec::new();
        self.player_defined_delayed_triggers.retain(|(p, delayed)| {
            if *p == player {
                to_activate.push(delayed.clone());
                false
            } else {
                true
            }
        });
        for delayed in to_activate {
            self.delayed_triggers.push(delayed);
        }
    }

    /// Mirrors Java's resetActiveTriggers().
    /// Scans all cards in game, collects active triggers.
    pub fn reset_active_triggers(&mut self, game: &GameState) {
        self.active_triggers.clear();
        for (i, card) in game.cards.iter().enumerate() {
            let card_id = CardId(i as u32);
            for (trig_idx, _) in card.triggers.iter().enumerate() {
                self.register_one_trigger(game, card_id, trig_idx);
            }
        }
    }

    /// Mirrors Java's registerActiveTrigger(card).
    /// Registers a single card's triggers.
    pub fn register_active_trigger(&mut self, game: &GameState, card_id: CardId) {
        let card = game.card(card_id);
        for (trig_idx, _) in card.triggers.iter().enumerate() {
            self.register_one_trigger(game, card_id, trig_idx);
        }
    }

    /// Java parity wrapper for TriggerHandler.clearActiveTriggers().
    pub fn clear_active_triggers(&mut self) {
        self.active_triggers.clear();
    }

    /// Java parity wrapper for TriggerHandler.registerActiveLTBTrigger(card).
    /// Registers all of a card's triggers as active, using current trigger indices.
    pub fn register_active_ltb_trigger(&mut self, game: &GameState, card_id: CardId) {
        let card = game.card(card_id);
        for (trig_idx, trigger) in card.triggers.iter().enumerate() {
            if self.looks_back_in_time(trigger) {
                self.register_one_trigger(game, card_id, trig_idx);
            }
        }
    }

    /// Force-register LTB triggers for a card that has already left the
    /// battlefield (e.g. during SBA batch processing).  Unlike
    /// `register_active_ltb_trigger`, this bypasses the zone check so that
    /// triggers from cards already in the graveyard can still see other
    /// creatures that die in the same SBA batch.
    /// Mirrors Java's behaviour where `triggerChangesZoneAll` re-registers
    /// LTB triggers from `lastStateBattlefield` before processing events.
    pub fn force_register_ltb_trigger(&mut self, game: &GameState, card_id: CardId) {
        let card = game.card(card_id);
        for (trig_idx, trigger) in card.triggers.iter().enumerate() {
            if self.looks_back_in_time(trigger) {
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

    /// Java parity wrapper for TriggerHandler.registerOneTrigger(...).
    pub fn register_one_trigger(
        &mut self,
        game: &GameState,
        card_id: CardId,
        trigger_index: usize,
    ) {
        let card = game.card(card_id);
        if trigger_index >= card.triggers.len() {
            return;
        }
        let trigger = &card.triggers[trigger_index];
        if !trigger.active_zones.contains(&card.zone) {
            return;
        }
        // NOTE: Do NOT call phases_check here.  Phase-gated triggers (e.g.
        // "At the beginning of your upkeep") must be registered as active
        // regardless of the current phase; the phase filter is evaluated at
        // match time inside can_run_trigger.
        if matches!(trigger.mode, TriggerMode::Always)
            && game.stack.has_state_trigger_id(trigger.id)
        {
            return;
        }
        let already_registered = self
            .active_triggers
            .iter()
            .any(|at| at.card_id == card_id && at.trigger_index == trigger_index);
        if !already_registered {
            self.active_triggers.push(ActiveTrigger {
                card_id,
                trigger_index,
            });
        }
    }

    /// Remove triggers for a card that left a trigger zone.
    pub fn unregister_active_triggers(&mut self, card_id: CardId) {
        self.active_triggers.retain(|at| at.card_id != card_id);
    }

    fn looks_back_in_time(&self, trigger: &Trigger) -> bool {
        if matches!(
            trigger.mode,
            TriggerMode::Exploited { .. }
                | TriggerMode::Destroyed { .. }
                | TriggerMode::Sacrificed { .. }
                | TriggerMode::SacrificedOnce { .. }
        ) {
            return true;
        }
        if matches!(
            trigger.mode,
            TriggerMode::ChangesZone { .. } | TriggerMode::ChangesZoneAll { .. }
        ) {
            let origin = trigger.params.get(keys::ORIGIN).unwrap_or("");
            let destination = trigger.params.get(keys::DESTINATION).unwrap_or("");
            return origin.contains("Battlefield")
                || destination.contains("Library")
                || destination.contains("Hand");
        }
        false
    }

    fn ltb_trigger_refs_for_event(
        &self,
        game: &GameState,
        event: &TriggerWaiting,
    ) -> Vec<(CardId, usize)> {
        match event.mode {
            TriggerType::ChangesZone => {
                let destination = event.params.destination;
                if event.params.origin != Some(forge_foundation::ZoneType::Battlefield)
                    && !matches!(
                        destination,
                        Some(
                            forge_foundation::ZoneType::Library | forge_foundation::ZoneType::Hand
                        )
                    )
                {
                    return Vec::new();
                }
                let Some(card_id) = event.params.card_lki.or(event.params.card) else {
                    return Vec::new();
                };
                game.card(card_id)
                    .triggers
                    .iter()
                    .enumerate()
                    .filter_map(|(trigger_index, trigger)| {
                        self.looks_back_in_time(trigger)
                            .then_some((card_id, trigger_index))
                    })
                    .collect()
            }
            TriggerType::ChangesZoneAll => event
                .params
                .change_zone_table
                .as_ref()
                .map(|table| {
                    table
                        .last_state_battlefield()
                        .iter()
                        .flat_map(|&card_id| {
                            game.card(card_id).triggers.iter().enumerate().filter_map(
                                move |(trigger_index, trigger)| {
                                    self.looks_back_in_time(trigger)
                                        .then_some((card_id, trigger_index))
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    pub fn clear_waiting_triggers(&mut self) {
        self.waiting_triggers.clear();
        self.pre_matched_triggers.clear();
    }

    pub fn on_player_lost(&mut self, player: PlayerId) {
        self.delayed_triggers.retain(|d| d.controller != player);
        self.this_turn_delayed_triggers
            .retain(|d| d.controller != player);
        self.player_defined_delayed_triggers
            .retain(|(p, _)| *p != player);
    }

    /// Mirrors Java's StaticAbilityDisableTriggers.disabled().
    /// Checks if a ChangesZone trigger is suppressed by a DisableTriggers
    /// static ability (e.g. Hushbringer).
    fn is_trigger_disabled_by_static(
        game: &GameState,
        host_card: CardId,
        trigger_index: usize,
        params: &RunParams,
    ) -> bool {
        let trigger = &game.card(host_card).triggers[trigger_index];

        // Only applies to ChangesZone triggers
        let _trigger_is_changes_zone = match &trigger.mode {
            TriggerMode::ChangesZone { .. } => true,
            _ => return false,
        };

        // The card changing zones (the "cause")
        let cause_card_id = match params.card {
            Some(cid) => cid,
            None => return false,
        };

        // For LTB (origin=Battlefield), use the LKI card state.
        // For ETB (destination=Battlefield), use current state.
        let cause_is_creature = if params.origin == Some(ZoneType::Battlefield) {
            // Card may have already moved — check if it WAS a creature
            // (LKI). The card's type_line is preserved after move_card.
            game.card(cause_card_id).is_creature()
        } else {
            game.card(cause_card_id).is_creature()
        };

        // Check cards for DisableTriggers static abilities.
        // For LTB triggers (Origin=Battlefield), also check cards that were on the
        // battlefield at the start of the current SBA check (pre_sba_battlefield).
        // This handles the case where a DisableTriggers source (e.g. Hushbringer)
        // dies in the same SBA batch as the trigger source — Hushbringer was on the
        // battlefield when the batch started, so its DisableTriggers still applies.
        // Mirrors Java's StaticAbilityDisableTriggers.disabled() which uses
        // lastStateBattlefield for LTB triggers (lines 20-27).
        let is_ltb = params.origin == Some(ZoneType::Battlefield);
        for card in game.cards.iter() {
            let on_battlefield = card.zone == ZoneType::Battlefield;
            let was_on_battlefield = is_ltb && game.pre_sba_battlefield.contains(&card.id);
            if !on_battlefield && !was_on_battlefield {
                continue;
            }
            for sa in &card.static_abilities {
                if sa.mode != crate::staticability::StaticMode::DisableTriggers {
                    continue;
                }

                // ValidCause$ — must match the card changing zones
                if let Some(valid_cause) = sa.params.get(keys::VALID_CAUSE) {
                    if valid_cause == "Creature" && !cause_is_creature {
                        continue;
                    }
                }

                // ValidMode$ — must match the trigger's mode
                if let Some(valid_mode) = sa.params.get(keys::VALID_MODE) {
                    let modes: Vec<&str> = valid_mode.split(',').collect();
                    if !modes.iter().any(|m| *m == "ChangesZone") {
                        continue;
                    }
                }

                // Origin$ — the event's origin zone must match
                if let Some(origin) = sa.params.get(keys::ORIGIN) {
                    let event_origin = params
                        .origin
                        .map(|z| format!("{:?}", z))
                        .unwrap_or_default();
                    if !origin.eq_ignore_ascii_case(&event_origin) {
                        continue;
                    }
                }

                // Destination$ — the event's destination zone must match
                if let Some(dest) = sa.params.get(keys::DESTINATION) {
                    let event_dest = params
                        .destination
                        .map(|z| format!("{:?}", z))
                        .unwrap_or_default();
                    if !dest.eq_ignore_ascii_case(&event_dest) {
                        continue;
                    }
                }

                // All conditions matched — trigger is disabled.
                return true;
            }
        }
        false
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
        let trigger_type = trigger.mode.trigger_type();

        let normalize = |t: TriggerType| -> TriggerType {
            match t {
                TriggerType::DungeonCompleted => TriggerType::CompletedDungeon,
                TriggerType::RoomEntered => TriggerType::EnteredRoom,
                TriggerType::Airbend
                | TriggerType::Earthbend
                | TriggerType::Firebend
                | TriggerType::Waterbend
                | TriggerType::ElementalBend => TriggerType::Elementalbend,
                TriggerType::TakesInitiative => TriggerType::TakeInitiative,
                TriggerType::PhaseIn => TriggerType::PhasedIn,
                TriggerType::PhaseOut => TriggerType::PhasedOut,
                TriggerType::PlaneswalkedFrom | TriggerType::PlaneswalkedTo => {
                    TriggerType::Planeswalk
                }
                TriggerType::CrankContraption => TriggerType::CrankAdvanced,
                TriggerType::Explores => TriggerType::Explored,
                TriggerType::Unattach => TriggerType::Unattached,
                TriggerType::Saddled | TriggerType::Stationed => TriggerType::Crewed,
                other => other,
            }
        };
        let mode_matches = normalize(trigger_type) == normalize(*mode);
        if !mode_matches {
            return false;
        }

        // Check suppression
        if self.is_trigger_suppressed(*mode) {
            return false;
        }

        // Common trigger phase/requirement/limit checks (Java Trigger base behavior).
        if !trigger.phases_check(game, host_card) {
            return false;
        }
        if !trigger.requirements_check(game, host_card) {
            return false;
        }
        if !trigger.check_activation_limit(game, host_card) {
            return false;
        }

        // DisableTriggers static ability check (e.g. Hushbringer).
        // Mirrors Java's StaticAbilityDisableTriggers.disabled().
        if matches!(
            *mode,
            TriggerType::ChangesZone | TriggerType::ChangesZoneAll
        ) {
            if Self::is_trigger_disabled_by_static(game, host_card, trigger_index, params) {
                return false;
            }
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
        } else if *mode == TriggerType::ChangesZone
            && params.origin == Some(ZoneType::Battlefield)
            && trigger.active_zones.contains(&ZoneType::Battlefield)
            && card.zone != ZoneType::Battlefield
            && self.looks_back_in_time(trigger)
        {
            // LKI active-zone check for LTB triggers seeing OTHER creatures die
            // in the same SBA batch (e.g. Blood Artist seeing Savannah Lions die).
            // The trigger's host card already left the battlefield but was
            // force-registered as an LTB trigger.
            ZoneType::Battlefield
        } else if (*mode == TriggerType::DamageDone || *mode == TriggerType::DamageDoneOnce)
            && params.damage_target_card == Some(host_card)
            && trigger.active_zones.contains(&ZoneType::Battlefield)
            && card.zone != ZoneType::Battlefield
        {
            // LKI for DamageDone/DamageDoneOnce triggers targeting self (e.g.
            // Raptor Hatchling enrage). When damage kills the creature, SBAs
            // move it to graveyard before triggers are processed. The trigger
            // was queued while the card was on the battlefield, so we use LKI.
            ZoneType::Battlefield
        } else if *mode == TriggerType::Exploited
            && params.card == Some(host_card)
            && trigger.active_zones.contains(&ZoneType::Battlefield)
            && card.zone != ZoneType::Battlefield
        {
            // LKI for Exploited triggers: the exploiting creature sacrificed
            // itself. It was on the battlefield when Exploit fired, so use LKI.
            ZoneType::Battlefield
        } else if *mode == TriggerType::Sacrificed
            && params.card == Some(host_card)
            && trigger.active_zones.contains(&ZoneType::Battlefield)
            && card.zone != ZoneType::Battlefield
        {
            // LKI for Sacrificed triggers: the creature was on the battlefield
            // when it was sacrificed, use LKI.
            ZoneType::Battlefield
        } else {
            card.zone
        };
        if !trigger.active_zones.contains(&zone_for_active_check) {
            return false;
        }

        // performTest
        if !trigger
            .mode
            .perform_test(params, game, host_card, host_controller, Some(trigger.id))
        {
            return false;
        }
        if !trigger.meets_requirements_on_triggered_objects(game, params, host_card) {
            return false;
        }

        // ── ActivatorThisTurnCast$ condition ──────────────────────────
        // Mirrors Java's TriggerSpellAbilityCast.checkActivatorThisTurnCast():
        // "EQ2" means the activating player must have cast exactly 2 spells
        // this turn (including the one triggering).
        if let Some(cond) = trigger.params.get(keys::ACTIVATOR_THIS_TURN_CAST) {
            let caster = params.spell_controller.unwrap_or(host_controller);
            let count = game.player(caster).spells_cast_this_turn;
            if !compare_expr(count, cond.trim()) {
                return false;
            }
        }

        // Mirrors Java Trigger.requirementsCheck() -> meetsCommonRequirements():
        // apply common IsPresent$/PresentCompare$/PresentPlayer$/PresentZone$ checks.
        if !valid_filter::check_is_present(game, &trigger.params, card) {
            return false;
        }

        true
    }

    pub fn suppress_mode(&mut self, mode: TriggerType) {
        self.suppressed_modes.insert(mode);
    }

    pub fn set_suppress_all_triggers(&mut self, suppress: bool) {
        self.all_suppressed = suppress;
    }

    /// Mirrors Java's TriggerHandler.getActiveTrigger(TriggerType, Map<AbilityKey, Object>).
    /// Returns all active triggers that can run for the given mode and params.
    pub fn get_active_trigger(
        &self,
        game: &GameState,
        mode: TriggerType,
        params: &RunParams,
    ) -> Vec<Trigger> {
        let mut result = Vec::new();
        for at in &self.active_triggers {
            let card = game.card(at.card_id);
            if at.trigger_index >= card.triggers.len() {
                continue;
            }
            let host_controller = card.controller;
            if self.can_run_trigger(
                game,
                at.card_id,
                at.trigger_index,
                host_controller,
                &mode,
                params,
            ) {
                result.push(card.triggers[at.trigger_index].clone());
            }
        }
        result
    }

    pub fn is_trigger_suppressed(&self, mode: TriggerType) -> bool {
        self.all_suppressed || self.suppressed_modes.contains(&mode)
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
