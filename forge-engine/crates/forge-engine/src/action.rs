use forge_foundation::ZoneType;

use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::replacement::handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::staticability::layer::apply_etb_tapped;
use crate::trigger::handler::TriggerHandler;

/// Game state mutation methods — moving cards, dealing damage, state-based actions.
impl GameState {
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

        let host_left_battlefield = src_zone == ZoneType::Battlefield && dest_zone != ZoneType::Battlefield;
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
                let etb_keywords = self.cards[card_id.index()].keywords.clone();
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
                return;
            }
            ZoneType::Graveyard | ZoneType::Hand | ZoneType::Exile | ZoneType::Library => {
                // Detach any attachments before resetting state.
                let attachments: Vec<CardId> = self.cards[card_id.index()].attachments.clone();
                for aura_id in attachments {
                    self.cards[aura_id.index()].attached_to = None;
                }
                self.cards[card_id.index()].attachments.clear();
                // Also detach this card from its host if it was an Aura/Equipment.
                self.detach(card_id);

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
                card.controller = card.owner;
                if !keep_counters {
                    card.counters.clear();
                }
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
            }
        }
    }

    /// Deal damage to a player.
    ///
    /// Runs replacement effects (e.g. damage prevention) before applying.
    /// Mirrors Java `GameAction.addDamage()` calling `ReplacementHandler.run()`.
    pub fn deal_damage_to_player(&mut self, target: PlayerId, amount: i32) {
        if amount <= 0 {
            return;
        }
        if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_lose_life(self, target) {
            return;
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
            }
        }
    }

    /// Check and apply state-based actions. Returns true if any were applied.
    pub fn check_state_based_actions(&mut self) -> bool {
        self.check_state_based_actions_with_triggers(None)
    }

    /// Check and apply state-based actions. Returns true if any were applied.
    /// If provided, emits ChangesZone triggers for SBA zone moves.
    pub fn check_state_based_actions_with_triggers(
        &mut self,
        mut trigger_handler: Option<&mut TriggerHandler>,
    ) -> bool {
        let mut any_changes = false;

        // Check players with 0 or less life
        for pid in self.player_order.clone() {
            if self.player(pid).life <= 0 && self.player(pid).is_alive() {
                let mut event = ReplacementEvent::GameLoss { player: pid };
                let result = apply_replacements(self, &mut event);
                if result != ReplacementResult::Replaced {
                    self.player_mut(pid).has_lost = true;
                    any_changes = true;
                }
            }
            // Check poison counters (10+ = lose)
            if self.player(pid).poison_counters >= 10 && self.player(pid).is_alive() {
                let mut event = ReplacementEvent::GameLoss { player: pid };
                let result = apply_replacements(self, &mut event);
                if result != ReplacementResult::Replaced {
                    self.player_mut(pid).has_lost = true;
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
                    self.player_mut(pid).has_lost = true;
                    any_changes = true;
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
            let card = &self.cards[cid.index()];
            if card.is_creature() {
                let zero_toughness = card.toughness() <= 0;
                let lethal =
                    card.lethal_damage() || (card.damage > 0 && card.has_deathtouch_damage);
                let should_die = zero_toughness || lethal;
                if should_die {
                    let owner = card.owner;
                    // CR 702.12: Indestructible prevents death from lethal damage and
                    // "destroy" effects, but NOT from toughness ≤ 0 (CR 704.5f vs 704.5g).
                    // This covers K:Indestructible from Forge card scripts (e.g. Darksteel Myr).
                    if lethal
                        && !zero_toughness
                        && self.cards[cid.index()].has_keyword("Indestructible")
                    {
                        continue;
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
                        self.move_card(cid, final_dest, owner);
                        if let Some(handler) = trigger_handler.as_deref_mut() {
                            crate::ability::effects::emit_zone_trigger(
                                handler, cid, old_zone, final_dest,
                            );
                        }
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
                    self.card_mut(cid).remove_counter(&CounterType::P1P1, cancel);
                    self.card_mut(cid).remove_counter(&CounterType::M1M1, cancel);
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
                for cid in ids.into_iter().skip(1) {
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
    pub fn untap_all(&mut self, player: PlayerId) {
        let cards: Vec<CardId> = self.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        for cid in cards {
            self.cards[cid.index()].tapped = false;
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

        let all_card_ids: Vec<CardId> = (0..self.cards.len()).map(|i| CardId(i as u32)).collect();
        for cid in all_card_ids {
            if self.cards[cid.index()].controller == player {
                self.cards[cid.index()].new_turn();
            }
        }
    }

    /// Tap a card. Returns true if it was untapped.
    pub fn tap(&mut self, card_id: CardId) -> bool {
        let card = &mut self.cards[card_id.index()];
        if !card.tapped {
            card.tapped = true;
            true
        } else {
            false
        }
    }

    /// Untap a card. Returns true if it was tapped.
    pub fn untap(&mut self, card_id: CardId) -> bool {
        let card = &mut self.cards[card_id.index()];
        if card.tapped {
            card.tapped = false;
            true
        } else {
            false
        }
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
        }
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
    use crate::card::CardInstance;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    fn make_creature(game: &mut GameState, name: &str, owner: PlayerId, p: i32, t: i32) -> CardId {
        let card = CardInstance::new(
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
