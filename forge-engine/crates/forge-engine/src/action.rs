use forge_foundation::ZoneType;

use crate::card::Card;
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::GameLossReason;
use crate::replacement::ReplacementResult;
use crate::staticability::layer::apply_etb_tapped;
use crate::trigger::handler::TriggerHandler;
use crate::trigger::parse_trigger;

/// Game state mutation methods — moving cards, dealing damage, state-based actions.
impl GameState {
    fn ensure_speed_effect(
        &mut self,
        player: PlayerId,
        trigger_handler: Option<&mut TriggerHandler>,
    ) {
        if self.player(player).speed == 0 || self.player(player).speed_effect_card.is_some() {
            return;
        }

        let mut effect = Card::new(
            CardId(0),
            "Start Your Engines!".to_string(),
            player,
            forge_foundation::CardTypeLine::parse("Effect"),
            forge_foundation::ManaCost::parse("0"),
            forge_foundation::ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        effect.set_controller(player);
        effect.set_s_var("SpeedUp", "DB$ ChangeSpeed");

        let mut next_trigger_id = 0;
        if let Some(trigger) = parse_trigger(
            "Mode$ LifeLostAll | ValidPlayer$ Opponent | TriggerZones$ Command | ActivationLimit$ 1 | PlayerTurn$ True | CheckSVar$ Count$YourSpeed | SVarCompare$ LT4 | Execute$ SpeedUp | TriggerDescription$ Whenever one or more opponents lose life during your turn, if your speed is less than 4, your speed increases by 1. This ability triggers only once each turn.",
            &mut next_trigger_id,
        ) {
            effect.add_trigger(trigger);
        }

        let effect_id = self.create_card(effect);
        self.move_card(effect_id, ZoneType::Command, player);
        self.player_mut(player).speed_effect_card = Some(effect_id);

        if let Some(handler) = trigger_handler {
            handler.register_active_trigger(self, effect_id);
        }
    }

    pub fn set_player_speed(
        &mut self,
        player: PlayerId,
        speed: i32,
        trigger_handler: Option<&mut TriggerHandler>,
    ) {
        let speed = speed.clamp(0, 4);
        self.player_mut(player).speed = speed;
        if speed > 0 {
            self.ensure_speed_effect(player, trigger_handler);
        }
    }

    pub fn increase_player_speed(
        &mut self,
        player: PlayerId,
        trigger_handler: Option<&mut TriggerHandler>,
    ) {
        let current = self.player(player).speed;
        if current < 4 {
            self.set_player_speed(player, current + 1, trigger_handler);
        }
    }

    pub fn decrease_player_speed(
        &mut self,
        player: PlayerId,
        trigger_handler: Option<&mut TriggerHandler>,
    ) {
        let current = self.player(player).speed;
        if current > 1 {
            self.set_player_speed(player, current - 1, trigger_handler);
        }
    }

    pub fn record_player_damage_assignment(
        &mut self,
        source: Option<CardId>,
        target_player: Option<PlayerId>,
        amount: i32,
        is_combat: bool,
    ) {
        if amount <= 0 {
            return;
        }
        let Some(source_id) = source else {
            return;
        };
        let controller = self.card(source_id).controller;
        {
            let controller_state = self.player_mut(controller);
            controller_state.assigned_damage_this_turn += amount;
            if is_combat {
                controller_state.assigned_combat_damage_this_turn += amount;
            }
        }
        if let Some(target) = target_player {
            if target != controller {
                self.player_mut(controller)
                    .opponents_assigned_damage_this_turn += amount;
            }
            if is_combat {
                self.player_mut(target)
                    .been_dealt_combat_damage_since_last_turn = true;
            }
        }
    }

    /// Move a card from its current zone to a new zone.
    pub fn move_card(&mut self, card_id: CardId, dest_zone: ZoneType, dest_owner: PlayerId) {
        // Commander redirect: commanders going to GY or Exile return to Command zone instead.
        // Mirrors Java's GameAction.stateBasedAction_Commander().
        let (dest_zone, dest_owner) = if self.cards[card_id.index()].is_commander
            && (dest_zone == ZoneType::Graveyard || dest_zone == ZoneType::Exile)
        {
            let owner = self.cards[card_id.index()].owner;
            (ZoneType::Command, owner)
        } else {
            (dest_zone, dest_owner)
        };

        let card = &self.cards[card_id.index()];
        let src_zone = card.zone;
        let src_owner = card.controller;

        let host_left_battlefield =
            src_zone == ZoneType::Battlefield && dest_zone != ZoneType::Battlefield;
        let forget_effects: Vec<CardId> = self
            .cards
            .iter()
            .filter(|c| {
                c.zone == ZoneType::Command
                    && c.forget_on_moved_origin == Some(src_zone)
                    && c.remembered_cards.contains(&card_id)
            })
            .map(|c| c.id)
            .collect();

        // Tokens and copy-tokens cease to exist when leaving the battlefield (CR 110.5g).
        // Set zone to None (limbo) and remove from source zone without adding to destination.
        if card.is_token && dest_zone != ZoneType::Battlefield {
            if let Some(table) = self.pending_change_zone_table.as_mut() {
                table.put(Some(src_zone), Some(ZoneType::None), card_id);
            }
            let mut exile_effects = Vec::new();
            for eff_id in forget_effects.iter().copied() {
                let eff = &mut self.cards[eff_id.index()];
                eff.remembered_cards.retain(|&rid| rid != card_id);
                if eff.exile_when_no_remembered && eff.remembered_cards.is_empty() {
                    exile_effects.push(eff_id);
                }
            }
            self.cards[card_id.index()].zone = ZoneType::None;
            if src_zone != ZoneType::None {
                self.zone_mut(src_zone, src_owner).remove(card_id);
            }
            // Effect cards with ForgetOnMoved should be removed from the game
            // entirely (zone = None), not moved to Exile. Moving them to Exile
            // creates phantom cards that diverge from Java parity.
            for eff_id in exile_effects {
                let controller = self.card(eff_id).controller;
                self.zone_mut(ZoneType::Command, controller).remove(eff_id);
                self.cards[eff_id.index()].zone = ZoneType::None;
            }
            return;
        }

        // Remove from source zone
        if src_zone != ZoneType::None {
            self.zone_mut(src_zone, src_owner).remove(card_id);
        }

        // Update card's zone
        self.cards[card_id.index()].zone = dest_zone;

        if let Some(table) = self.pending_change_zone_table.as_mut() {
            table.put(Some(src_zone), Some(dest_zone), card_id);
        }

        // Assign a zone timestamp so same-player triggers are ordered by
        // zone entry order (matching Java's Zone.cardList insertion order).
        self.assign_zone_timestamp(card_id);

        // Track LKI: record which zone this card came from on the destination zone.
        self.zone_mut(dest_zone, dest_owner)
            .save_lki(card_id, src_zone);

        // Reset state on zone change
        match dest_zone {
            ZoneType::Battlefield => {
                self.cards[card_id.index()].enter_battlefield();
                // Add to destination zone first so the card is "on the
                // battlefield" when ETB-tapped checks run against it.
                self.zone_mut(dest_zone, dest_owner).add(card_id);
                // Apply ETB-tapped effects (intrinsic + extrinsic).
                apply_etb_tapped(self, card_id);
                // Keyword ETB counters: K:etbCounter:TYPE:N
                let etb_keywords = self.cards[card_id.index()].keywords.as_string_list();
                for kw in etb_keywords {
                    let mut parts = kw.split(':');
                    let head = parts.next().unwrap_or_default();
                    if !head.eq_ignore_ascii_case("etbCounter") {
                        continue;
                    }
                    let counter_type = parts.next().unwrap_or_default();
                    let amount = parts
                        .next()
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0);
                    if amount <= 0 {
                        continue;
                    }
                    let ct = crate::ability::effects::parse_counter_type(counter_type);
                    // Respect CantPutCounter (e.g. Solemnity) before placing ETB counters.
                    if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                        &self.cards,
                        &self.cards[card_id.index()],
                        &ct,
                    ) {
                        self.cards[card_id.index()].add_counter(&ct, amount);
                    }
                }
                // Apply +1/+1 counters from mana that adds counters (Guildmages' Forum, Opal Palace)
                let etb_p1p1 = self.cards[card_id.index()].etb_counters_p1p1;
                if etb_p1p1 > 0 {
                    let ct = crate::card::CounterType::P1P1;
                    if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                        &self.cards,
                        &self.cards[card_id.index()],
                        &ct,
                    ) {
                        self.cards[card_id.index()].add_counter(&ct, etb_p1p1);
                    }
                    self.cards[card_id.index()].etb_counters_p1p1 = 0;
                }
                // Sunburst: add counters based on colors of mana spent
                let sunburst = self.cards[card_id.index()].sunburst_count();
                if sunburst > 0 && self.cards[card_id.index()].has_keyword("Sunburst") {
                    let ct = if self.cards[card_id.index()].is_creature() {
                        crate::card::CounterType::P1P1
                    } else {
                        crate::card::CounterType::Charge
                    };
                    if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                        &self.cards,
                        &self.cards[card_id.index()],
                        &ct,
                    ) {
                        self.cards[card_id.index()].add_counter(&ct, sunburst);
                    }
                }
                // Update LKI snapshot: card just entered the battlefield.
                // Ensures it's available for later TriggeredCard$CardPower lookups
                // even if it dies within the same resolution chain.
                self.update_lki_snapshot(card_id);
                return;
            }
            ZoneType::Graveyard | ZoneType::Hand | ZoneType::Exile | ZoneType::Library => {
                // Detach any attachments before resetting state.
                let attachments: Vec<CardId> = self.cards[card_id.index()].attachments.clone();
                for aura_id in attachments {
                    self.cards[aura_id.index()].attached_to = None;
                    // Bestow: when host leaves, revert aura to creature
                    self.cards[aura_id.index()].is_bestowed = false;
                }
                self.cards[card_id.index()].attachments.clear();
                // Also detach this card from its host if it was an Aura/Equipment.
                self.detach(card_id);

                // Save last-known information before resetting.
                // Mirrors Java's LKI system for trigger SVars like TriggeredCard$CardPower.
                if src_zone == ZoneType::Battlefield {
                    let card = &self.cards[card_id.index()];
                    let lki_p = card.power();
                    let lki_t = card.toughness();
                    let card = &mut self.cards[card_id.index()];
                    card.lki_power = Some(lki_p);
                    card.lki_toughness = Some(lki_t);
                }

                // Reset battlefield state when leaving (including static modifiers).
                let keep_counters =
                    crate::staticability::static_ability_counters_remain::counters_remain(
                        &self.cards,
                        &self.cards[card_id.index()],
                        dest_zone,
                    );
                let card = &mut self.cards[card_id.index()];
                card.tapped = false;
                card.damage = 0;
                card.power_modifier = 0;
                card.toughness_modifier = 0;
                card.static_power_modifier = 0;
                card.static_toughness_modifier = 0;
                card.static_set_power = None;
                card.static_set_toughness = None;
                card.granted_keywords.clear();
                card.cant_attack_static = false;
                card.cant_block_static = false;
                card.summoning_sick = true;
                card.monstrous = false;
                card.controller = card.owner;
                card.face_down = false;
                card.is_bestowed = false;
                if !keep_counters {
                    card.counters.clear();
                }
                // Clear temporary triggers added by Animate effects (e.g.
                // Supernatural Stamina's "when this creature dies, return it").
                // Per CR 400.7 a permanent that changes zones becomes a new
                // object; it must not retain one-shot death-return triggers.
                // Without this, a creature that dies-and-returns would still
                // carry the trigger, making it "immortal" for the rest of the
                // turn.
                card.clear_pump_triggers();
            }
            ZoneType::Command => {
                // Detach any attachments before resetting state.
                let attachments: Vec<CardId> = self.cards[card_id.index()].attachments.clone();
                for aura_id in attachments {
                    self.cards[aura_id.index()].attached_to = None;
                }
                self.cards[card_id.index()].attachments.clear();
                self.detach(card_id);

                // Commander returning to command zone: reset battlefield state.
                let keep_counters =
                    crate::staticability::static_ability_counters_remain::counters_remain(
                        &self.cards,
                        &self.cards[card_id.index()],
                        dest_zone,
                    );
                let card = &mut self.cards[card_id.index()];
                card.tapped = false;
                card.damage = 0;
                card.power_modifier = 0;
                card.toughness_modifier = 0;
                card.static_power_modifier = 0;
                card.static_toughness_modifier = 0;
                card.static_set_power = None;
                card.static_set_toughness = None;
                card.granted_keywords.clear();
                card.cant_attack_static = false;
                card.cant_block_static = false;
                card.summoning_sick = true;
                card.monstrous = false;
                card.controller = card.owner;
                if !keep_counters {
                    card.counters.clear();
                }
            }
            _ => {}
        }

        // Add to destination zone
        self.zone_mut(dest_zone, dest_owner).add(card_id);

        // Forget remembered objects for command effects with ForgetOnMoved.
        let mut exile_effects = Vec::new();
        for eff_id in forget_effects {
            let eff = &mut self.cards[eff_id.index()];
            eff.remembered_cards.retain(|&rid| rid != card_id);
            if eff.exile_when_no_remembered && eff.remembered_cards.is_empty() {
                exile_effects.push(eff_id);
            }
        }
        // Effect cards with ForgetOnMoved should be removed from the game
        // entirely (zone = None), not moved to Exile.
        for eff_id in exile_effects {
            let controller = self.card(eff_id).controller;
            self.zone_mut(ZoneType::Command, controller).remove(eff_id);
            self.cards[eff_id.index()].zone = ZoneType::None;
        }

        // Expire temporary effect cards linked to this host leaving play
        // (Duration$ UntilHostLeavesPlay / UntilHostLeavesPlayOrEOT).
        if host_left_battlefield {
            let linked_effects: Vec<CardId> = self
                .cards
                .iter()
                .filter(|c| c.zone == ZoneType::Command && c.temp_effect_host == Some(card_id))
                .map(|c| c.id)
                .collect();
            for eff_id in linked_effects {
                let controller = self.card(eff_id).controller;
                self.zone_mut(ZoneType::Command, controller).remove(eff_id);
                self.cards[eff_id.index()].zone = ZoneType::None;
            }

            // Return cards exiled by this host via ChangeZoneAll Duration$ UntilHostLeavesPlay
            // (e.g. Deputy of Detention: exiled permanents return when it leaves).
            let exiled_by_host: Vec<(CardId, PlayerId)> = self
                .cards
                .iter()
                .filter(|c| c.zone == ZoneType::Exile && c.exiled_by == Some(card_id))
                .map(|c| (c.id, c.owner))
                .collect();
            for (exiled_id, owner) in exiled_by_host {
                self.cards[exiled_id.index()].exiled_by = None;
                self.move_card(exiled_id, ZoneType::Battlefield, owner);
            }
        }
    }

    /// Deal damage to a card (creature).
    ///
    /// Runs replacement effects (e.g. damage prevention) before applying.
    /// Mirrors Java `GameAction.addDamage()` calling `ReplacementHandler.run()`.
    pub fn deal_damage_to_card(&mut self, target: CardId, amount: i32) {
        if amount <= 0 {
            return;
        }
        let mut event = ReplacementEvent::DamageToCard {
            target,
            amount,
            source: None,
            is_combat: false,
        };
        apply_replacements(self, &mut event);
        if let ReplacementEvent::DamageToCard {
            amount: final_amount,
            ..
        } = event
        {
            if final_amount > 0 {
                self.cards[target.index()].damage += final_amount;
                // Fire DealtDamage replacement event after damage is applied.
                let mut dealt_event = ReplacementEvent::DealtDamage {
                    target,
                    amount: final_amount,
                    source: None,
                };
                apply_replacements(self, &mut dealt_event);
            }
        }
    }

    /// Deal damage to a player.
    ///
    /// Runs replacement effects (e.g. damage prevention) before applying.
    /// Mirrors Java `GameAction.addDamage()` calling `ReplacementHandler.run()`.
    pub fn deal_damage_to_player(&mut self, target: PlayerId, amount: i32) -> i32 {
        if amount <= 0 {
            return 0;
        }
        if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_lose_life(
            self, target,
        ) {
            return 0;
        }
        let mut event = ReplacementEvent::DamageToPlayer {
            target,
            amount,
            source: None,
            is_combat: false,
        };
        apply_replacements(self, &mut event);
        if let ReplacementEvent::DamageToPlayer {
            amount: final_amount,
            ..
        } = event
        {
            if final_amount > 0 {
                self.players[target.index()].deal_damage(final_amount);
                return final_amount;
            }
        }
        0
    }

    /// Check and apply state-based actions. Returns true if any were applied.
    pub fn check_state_based_actions(&mut self) -> bool {
        self.check_state_based_actions_with_triggers(None, None)
    }

    /// Check and apply state-based actions. Returns true if any were applied.
    /// If provided, emits ChangesZone triggers for SBA zone moves.
    /// `legend_keep_fn` — optional callback for legend rule: given (player, duplicates),
    /// returns the CardId to keep.  Mirrors Java's `chooseSingleEntityForEffect`.
    pub fn check_state_based_actions_with_triggers(
        &mut self,
        mut trigger_handler: Option<&mut TriggerHandler>,
        mut legend_keep_fn: Option<&mut dyn FnMut(PlayerId, &[CardId]) -> CardId>,
    ) -> bool {
        // Capture battlefield state before SBA processing. Used by DisableTriggers
        // (Hushbringer) to check LKI — if a creature with DisableTriggers dies in
        // the same SBA batch as another creature, it still suppresses death triggers.
        // Mirrors Java's LastStateBattlefield passed through RunParams.
        self.pre_sba_battlefield = self
            .cards
            .iter()
            .filter(|c| c.zone == ZoneType::Battlefield)
            .map(|c| c.id)
            .collect();

        let mut any_changes = false;
        let mut newly_lost_players: Vec<PlayerId> = Vec::new();

        // Check players with 0 or less life
        for pid in self.player_order.clone() {
            if self.player(pid).life <= 0 && self.player(pid).is_alive() {
                let mut event = ReplacementEvent::GameLoss {
                    player: pid,
                    reason: GameLossReason::LifeReachedZero,
                };
                let result = apply_replacements(self, &mut event);
                if result != ReplacementResult::Replaced {
                    if !self.player(pid).has_lost {
                        self.player_mut(pid).has_lost = true;
                        newly_lost_players.push(pid);
                        any_changes = true;
                    }
                }
            }
            // Check poison counters (10+ = lose)
            if self.player(pid).poison_counters >= 10 && self.player(pid).is_alive() {
                let mut event = ReplacementEvent::GameLoss {
                    player: pid,
                    reason: GameLossReason::Poisoned,
                };
                let result = apply_replacements(self, &mut event);
                if result != ReplacementResult::Replaced {
                    let mut event = ReplacementEvent::GameLoss {
                        player: pid,
                        reason: GameLossReason::CommanderDamage,
                    };
                    let result = apply_replacements(self, &mut event);
                    if result != ReplacementResult::Replaced {
                        if !self.player(pid).has_lost {
                            self.player_mut(pid).has_lost = true;
                            newly_lost_players.push(pid);
                        }
                    }
                    any_changes = true;
                }
            }
            // Check commander damage (21+ from a single commander source = lose)
            let commander_dmg_entries: Vec<(u32, i32)> = self
                .player(pid)
                .commander_damage_received
                .iter()
                .map(|(&k, &v)| (k, v))
                .collect();
            for (_card_raw_id, dmg) in commander_dmg_entries {
                if dmg >= 21 && self.player(pid).is_alive() {
                    if !self.player(pid).has_lost {
                        self.player_mut(pid).has_lost = true;
                        newly_lost_players.push(pid);
                        any_changes = true;
                    }
                }
            }

            // CR 704.5z: If a player controls a permanent with Start your
            // engines! and that player has no speed, their speed becomes 1.
            if self.player(pid).speed == 0
                && self
                    .cards_in_zone(ZoneType::Battlefield, pid)
                    .iter()
                    .any(|&cid| self.card(cid).has_keyword("Start your engines"))
            {
                self.increase_player_speed(pid, None);
                any_changes = true;
            }
        }

        if !newly_lost_players.is_empty() {
            for pid in &newly_lost_players {
                self.stack.remove_instances_controlled_by(*pid);
            }
            if let Some(handler) = trigger_handler.as_deref_mut() {
                for pid in &newly_lost_players {
                    handler.run_trigger(
                        TriggerType::LosesGame,
                        RunParams {
                            player: Some(*pid),
                            ..Default::default()
                        },
                        false,
                    );
                    handler.on_player_lost(*pid);
                }
            }
        }

        // Check creatures with lethal damage or 0 toughness
        let battlefield_cards: Vec<CardId> = self
            .player_order
            .clone()
            .iter()
            .flat_map(|&pid| self.cards_in_zone(ZoneType::Battlefield, pid).to_vec())
            .collect();

        for cid in battlefield_cards {
            let (is_creature, zero_toughness, lethal, should_die, owner) = {
                let card = &self.cards[cid.index()];
                let is_creature = card.is_creature();
                let zero_toughness = card.toughness() <= 0;
                let lethal = card.lethal_damage() || card.has_deathtouch_damage;
                let should_die = zero_toughness || lethal;
                (is_creature, zero_toughness, lethal, should_die, card.owner)
            };
            if is_creature {
                if should_die {
                    // Clear deathtouch flag regardless of outcome (mirrors Java
                    // GameAction.java line 1491: c.setHasBeenDealtDeathtouchDamage(false)).
                    self.cards[cid.index()].has_deathtouch_damage = false;
                    let owner = owner;
                    // CR 702.12: Indestructible prevents death from lethal damage and
                    // "destroy" effects, but NOT from toughness ≤ 0 (CR 704.5f vs 704.5g).
                    // This covers K:Indestructible from Forge card scripts (e.g. Darksteel Myr).
                    if lethal
                        && !zero_toughness
                        && self.cards[cid.index()].has_keyword("Indestructible")
                    {
                        continue;
                    }
                    // CR 702.89: Umbra armor (Totem Armor) — if enchanted creature
                    // would be destroyed, instead remove all damage and destroy the aura.
                    let has_umbra = self.cards[cid.index()]
                        .attachments
                        .iter()
                        .any(|&aid| {
                            aid.index() < self.cards.len()
                                && self.cards[aid.index()].zone == ZoneType::Battlefield
                                && (self.cards[aid.index()].has_keyword("Umbra armor")
                                    || self.cards[aid.index()].has_keyword("Totem armor"))
                        });
                    if has_umbra && !zero_toughness {
                        // Find the first umbra armor aura and destroy it instead
                        let umbra_id = self.cards[cid.index()]
                            .attachments
                            .iter()
                            .copied()
                            .find(|&aid| {
                                aid.index() < self.cards.len()
                                    && self.cards[aid.index()].zone == ZoneType::Battlefield
                                    && (self.cards[aid.index()].has_keyword("Umbra armor")
                                        || self.cards[aid.index()].has_keyword("Totem armor"))
                            });
                        if let Some(umbra_id) = umbra_id {
                            // Remove all damage from the creature
                            self.cards[cid.index()].damage = 0;
                            self.cards[cid.index()].has_deathtouch_damage = false;
                            // Destroy the aura instead
                            let umbra_owner = self.cards[umbra_id.index()].owner;
                            let old_zone = self.cards[umbra_id.index()].zone;
                            self.move_card(umbra_id, ZoneType::Graveyard, umbra_owner);
                            if let Some(handler) = trigger_handler.as_deref_mut() {
                                crate::ability::effects::emit_zone_trigger(
                                    handler, umbra_id, old_zone, ZoneType::Graveyard,
                                );
                            }
                            any_changes = true;
                            continue; // Creature survives
                        }
                    }

                    // Run Destroy replacement effects (R$-based indestructible, etc.).
                    // Mirrors Java GameAction.destroy() → ReplacementHandler.run(Destroy, …).
                    let mut destroy_event = ReplacementEvent::Destroy { target: cid };
                    let result = apply_replacements(self, &mut destroy_event);
                    if result != ReplacementResult::Replaced {
                        // No replacement blocked destruction — run Moved check in case
                        // a zone-rerouting effect applies (e.g. "exile instead of die").
                        let mut moved_event = ReplacementEvent::Moved {
                            card: cid,
                            origin: ZoneType::Battlefield,
                            destination: ZoneType::Graveyard,
                        };
                        apply_replacements(self, &mut moved_event);
                        let final_dest =
                            if let ReplacementEvent::Moved { destination, .. } = moved_event {
                                destination
                            } else {
                                ZoneType::Graveyard
                            };
                        let old_zone = self.card(cid).zone;
                        // Emit trigger BEFORE move_card so that LKI state
                        // (counters, keywords) is still available for trigger
                        // matching.  Persist/Undying check counter conditions
                        // on the dying card; if we emit after move_card the
                        // counters are already cleared and the check is wrong.
                        // flush_waiting_triggers pre-matches while card state
                        // is intact; the matched results survive the move.
                        if let Some(handler) = trigger_handler.as_deref_mut() {
                            // Capture +1/+1 counters for LKI (Modular death
                            // triggers).  Counters are still present since we
                            // emit before move_card.
                            let lki_p1p1 = *self
                                .card(cid)
                                .counters
                                .get(&crate::card::CounterType::P1P1)
                                .unwrap_or(&0);
                            crate::ability::effects::emit_zone_trigger_with_lki_counters(
                                handler, cid, old_zone, final_dest, lki_p1p1,
                            );
                            handler.flush_waiting_triggers(self);
                        }
                        self.move_card(cid, final_dest, owner);
                        any_changes = true;
                    } else {
                        // Indestructible — destruction was replaced; creature stays.
                        // Damage is still marked but the creature does not die.
                    }
                }
            }
        }

        // CR 704.5q: +1/+1 and -1/-1 counter cancellation
        for &pid in &self.player_order.clone() {
            let battlefield = self.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
            for cid in battlefield {
                let p1 = self.card(cid).counter_count(&CounterType::P1P1);
                let m1 = self.card(cid).counter_count(&CounterType::M1M1);
                if p1 > 0 && m1 > 0 {
                    let cancel = p1.min(m1);
                    self.card_mut(cid)
                        .remove_counter(&CounterType::P1P1, cancel);
                    self.card_mut(cid)
                        .remove_counter(&CounterType::M1M1, cancel);
                    any_changes = true;
                }
            }
        }

        // Legend rule: for each player, if they control multiple legendary
        // permanents with the same name, keep one and move the rest to graveyard.
        // IgnoreLegendRule statics exempt matching cards.
        for &pid in &self.player_order.clone() {
            let battlefield = self.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
            let mut by_name: std::collections::BTreeMap<String, Vec<CardId>> =
                std::collections::BTreeMap::new();
            for cid in battlefield {
                let c = self.card(cid);
                if !c.type_line.is_legendary() {
                    continue;
                }
                if crate::staticability::static_ability_ignore_legend_rule::ignore_legend_rule(
                    &self.cards,
                    c,
                ) {
                    continue;
                }
                by_name.entry(c.card_name.clone()).or_default().push(cid);
            }
            for (_name, ids) in by_name {
                if ids.len() <= 1 {
                    continue;
                }
                // Choose which to keep: delegate to callback (mirrors Java's
                // chooseSingleEntityForEffect), or default to first in zone order.
                let keep = if let Some(ref mut chooser) = legend_keep_fn {
                    chooser(pid, &ids)
                } else {
                    ids[0]
                };
                for cid in ids {
                    if cid == keep {
                        continue;
                    }
                    let owner = self.card(cid).owner;
                    let old_zone = self.card(cid).zone;
                    self.move_card(cid, ZoneType::Graveyard, owner);
                    if let Some(handler) = trigger_handler.as_deref_mut() {
                        crate::ability::effects::emit_zone_trigger(
                            handler,
                            cid,
                            old_zone,
                            ZoneType::Graveyard,
                        );
                    }
                    any_changes = true;
                }
            }
        }

        // CR 704.5n: Aura SBA — an Aura on the battlefield that is not attached
        // to a legal permanent (or whose host left the battlefield) is put into
        // its owner's graveyard.
        {
            let aura_ids: Vec<CardId> = self
                .cards
                .iter()
                .filter(|c| {
                    c.zone == ZoneType::Battlefield
                        && c.type_line.has_subtype("Aura")
                        && !c.type_line.is_creature() // Bestowed auras that became creatures stay
                })
                .filter(|c| {
                    match c.attached_to {
                        None => true, // Not attached to anything — orphaned
                        Some(host_id) => {
                            if host_id.index() >= self.cards.len() {
                                return true; // Invalid host ID
                            }
                            let host = &self.cards[host_id.index()];
                            if host.zone != ZoneType::Battlefield {
                                return true; // Host left the battlefield
                            }
                            // CR 704.5n: check if the enchant restriction is still met.
                            // E.g. "Enchant creature" — if the host is no longer a creature,
                            // the aura falls off.
                            let enchant_type = c
                                .keywords
                                .iter_strings()
                                .find_map(|kw| {
                                    crate::keyword::extract_keyword_cost_str(&kw, "Enchant")
                                })
                                .unwrap_or_default();
                            !crate::parsing::enchant_type_matches_card(&enchant_type, host)
                        }
                    }
                })
                .map(|c| c.id)
                .collect();

            for aura_id in aura_ids {
                let owner = self.card(aura_id).owner;
                let old_zone = self.card(aura_id).zone;
                self.move_card(aura_id, ZoneType::Graveyard, owner);
                if let Some(handler) = trigger_handler.as_deref_mut() {
                    crate::ability::effects::emit_zone_trigger(
                        handler,
                        aura_id,
                        old_zone,
                        ZoneType::Graveyard,
                    );
                }
                any_changes = true;
            }
        }

        // Check game over
        let alive = self.alive_players();
        if alive.len() <= 1 {
            self.game_over = true;
            if alive.len() == 1 {
                self.winner = Some(alive[0]);
            }
        }

        any_changes
    }

    /// Untap all permanents controlled by a player.
    /// Runs Untap replacement effects for each permanent.
    pub fn untap_all(&mut self, player: PlayerId) {
        let cards: Vec<CardId> = self.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        for cid in cards {
            // Use untap() which runs replacement effects
            self.untap(cid);
        }
    }

    /// Draw a card for a player. Returns the drawn card ID, or None if the draw
    /// was skipped or the library is empty.
    ///
    /// Runs Draw replacement effects before drawing.  If the draw is replaced
    /// (e.g. "skip your draw step"), returns `None`.
    ///
    /// Mirrors Java `GameAction.draw()` calling `ReplacementHandler.run(Draw, …)`.
    pub fn draw_card(&mut self, player: PlayerId) -> Option<CardId> {
        if crate::staticability::static_ability_cant_draw::can_draw_amount(self, player, 1) <= 0 {
            return None;
        }
        // Run Draw replacement effects.
        let mut event = ReplacementEvent::Draw { player };
        let result = apply_replacements(self, &mut event);
        if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
            return None;
        }

        let card_id = self.zone_mut(ZoneType::Library, player).take_top()?;
        self.move_card(card_id, ZoneType::Hand, player);
        self.player_mut(player).drawn_this_turn += 1;
        Some(card_id)
    }

    /// Draw N cards for a player. Returns drawn card IDs.
    pub fn draw_cards(&mut self, player: PlayerId, n: usize) -> Vec<CardId> {
        let mut drawn = Vec::new();
        for _ in 0..n {
            if let Some(cid) = self.draw_card(player) {
                drawn.push(cid);
            } else {
                // Drawing from empty library — player loses (handled by SBA)
                break;
            }
        }
        drawn
    }

    /// Shuffle a player's library using the provided RNG.
    pub fn shuffle_library(&mut self, player: PlayerId, rng: &mut impl rand::Rng) {
        use rand::seq::SliceRandom;
        let zone = self.zone_mut(ZoneType::Library, player);
        zone.cards.shuffle(rng);
    }

    /// Reset per-turn state for all cards and players of a given player.
    pub fn new_turn_for_player(&mut self, player: PlayerId) {
        self.player_mut(player).new_turn();
        // Reset drawn_this_turn for ALL players (mirrors Java Game.newTurn).
        // The Drawn trigger Number$ check requires an accurate per-turn count.
        for pid in &self.player_order.clone() {
            if *pid != player {
                self.player_mut(*pid).drawn_this_turn = 0;
            }
        }

        let all_card_ids: Vec<CardId> = (0..self.cards.len()).map(|i| CardId(i as u32)).collect();
        for cid in all_card_ids {
            if self.cards[cid.index()].controller == player {
                if self.cards[cid.index()].zone == ZoneType::Battlefield {
                    self.cards[cid.index()].started_turn_tapped = self.cards[cid.index()].tapped;
                }
                self.cards[cid.index()].new_turn();
            }
        }
    }

    /// Tap a card. Returns true if it was untapped.
    /// Runs Tap replacement effects before tapping.
    pub fn tap(&mut self, card_id: CardId) -> bool {
        let card = &self.cards[card_id.index()];
        if card.tapped {
            return false;
        }
        // Run Tap replacement effects.
        let mut event = ReplacementEvent::Tap { card: card_id };
        let result = apply_replacements(self, &mut event);
        if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
            return false; // Tap was prevented
        }
        self.cards[card_id.index()].tapped = true;
        true
    }

    /// Untap a card. Returns true if it was tapped.
    /// Runs Untap replacement effects before untapping.
    pub fn untap(&mut self, card_id: CardId) -> bool {
        let card = &self.cards[card_id.index()];
        if !card.tapped {
            return false;
        }
        // Run Untap replacement effects.
        let mut event = ReplacementEvent::Untap { card: card_id };
        let result = apply_replacements(self, &mut event);
        if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
            return false; // Untap was prevented
        }
        self.cards[card_id.index()].tapped = false;
        true
    }

    /// Change the controller of a permanent to `new_controller`.
    /// Mirrors Java's `GameAction.controllerChangeZoneCorrection()` — moves the
    /// card between per-player zone lists and updates the controller field.
    pub fn change_controller(&mut self, card_id: CardId, new_controller: PlayerId) {
        let card = &self.cards[card_id.index()];
        if card.controller == new_controller {
            return;
        }
        let old_controller = card.controller;
        let zone = card.zone;

        // Move between zone lists
        if zone != ZoneType::None {
            self.zone_mut(zone, old_controller).remove(card_id);
            self.zone_mut(zone, new_controller).add(card_id);
        }
        self.cards[card_id.index()].controller = new_controller;
    }

    /// Attach `aura_id` to `target_id`.
    /// If `aura_id` was already attached elsewhere, detach it first.
    /// Mirrors Java's `Card.enchantEntity()` / `Card.equip()`.
    pub fn attach_to(&mut self, aura_id: CardId, target_id: CardId) {
        // Detach from previous host if any
        self.detach(aura_id);
        self.cards[aura_id.index()].attached_to = Some(target_id);
        self.cards[target_id.index()].attachments.push(aura_id);
    }

    /// Detach `aura_id` from whatever it is currently attached to.
    /// Mirrors Java's `Card.unattachFromEntity()`.
    pub fn detach(&mut self, aura_id: CardId) {
        if let Some(host_id) = self.cards[aura_id.index()].attached_to.take() {
            self.cards[host_id.index()]
                .attachments
                .retain(|&a| a != aura_id);
            // Bestow: when unattached, revert to a creature
            self.cards[aura_id.index()].is_bestowed = false;
        }
    }

    /// Move a card from its current zone to the bottom of a player's library.
    /// Unlike `move_card`, this places the card at the bottom rather than the top.
    pub fn put_on_bottom_of_library(&mut self, card_id: CardId, owner: PlayerId) {
        let card = &self.cards[card_id.index()];
        let src_zone = card.zone;
        let src_owner = card.controller;

        if src_zone != ZoneType::None {
            self.zone_mut(src_zone, src_owner).remove(card_id);
        }

        self.cards[card_id.index()].zone = ZoneType::Library;
        self.assign_zone_timestamp(card_id);
        self.zone_mut(ZoneType::Library, owner)
            .add_to_bottom(card_id);
    }

    /// Remove a spell from the stack by its entry ID (used by Counter).
    /// Mirrors Java's `Game.getStack().remove(sa)`.
    pub fn remove_from_stack(&mut self, entry_id: u32) -> bool {
        self.stack.remove_by_id(entry_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    fn make_creature(game: &mut GameState, name: &str, owner: PlayerId, p: i32, t: i32) -> CardId {
        let card = Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(p),
            Some(t),
            vec![],
            vec![],
        );
        game.create_card(card)
    }

    #[test]
    fn move_card_to_battlefield() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let cid = make_creature(&mut game, "Bear", PlayerId(0), 2, 2);
        game.move_card(cid, ZoneType::Hand, PlayerId(0));
        assert_eq!(game.zone(ZoneType::Hand, PlayerId(0)).len(), 1);

        game.move_card(cid, ZoneType::Battlefield, PlayerId(0));
        assert_eq!(game.zone(ZoneType::Hand, PlayerId(0)).len(), 0);
        assert_eq!(game.zone(ZoneType::Battlefield, PlayerId(0)).len(), 1);
        assert_eq!(game.card(cid).zone, ZoneType::Battlefield);
    }

    #[test]
    fn state_based_actions_lethal_damage() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let cid = make_creature(&mut game, "Bear", PlayerId(0), 2, 2);
        game.move_card(cid, ZoneType::Battlefield, PlayerId(0));

        game.deal_damage_to_card(cid, 2);
        assert!(game.check_state_based_actions());
        assert_eq!(game.zone(ZoneType::Graveyard, PlayerId(0)).len(), 1);
    }

    #[test]
    fn state_based_actions_zero_life() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        game.deal_damage_to_player(PlayerId(0), 20);
        game.check_state_based_actions();
        assert!(game.player(PlayerId(0)).has_lost);
        assert!(game.game_over);
        assert_eq!(game.winner, Some(PlayerId(1)));
    }

    #[test]
    fn draw_card() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let cid = make_creature(&mut game, "Bear", PlayerId(0), 2, 2);
        game.move_card(cid, ZoneType::Library, PlayerId(0));

        let drawn = game.draw_card(PlayerId(0));
        assert_eq!(drawn, Some(cid));
        assert_eq!(game.card(cid).zone, ZoneType::Hand);
    }

    #[test]
    fn tap_untap() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let cid = make_creature(&mut game, "Bear", PlayerId(0), 2, 2);
        game.move_card(cid, ZoneType::Battlefield, PlayerId(0));

        assert!(game.tap(cid));
        assert!(game.card(cid).tapped);
        assert!(!game.tap(cid)); // already tapped
        assert!(game.untap(cid));
        assert!(!game.card(cid).tapped);
    }
}
