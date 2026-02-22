use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::staticability::layer::apply_etb_tapped;
use crate::replacement::ReplacementResult;
use crate::replacement::handler::{apply_replacements, ReplacementEvent};

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

        // Tokens and copy-tokens cease to exist when leaving the battlefield (CR 110.5g).
        // Set zone to None (limbo) and remove from source zone without adding to destination.
        if card.is_token && dest_zone != ZoneType::Battlefield {
            self.cards[card_id.index()].zone = ZoneType::None;
            if src_zone != ZoneType::None {
                self.zone_mut(src_zone, src_owner).remove(card_id);
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
            }
            _ => {}
        }

        // Add to destination zone
        self.zone_mut(dest_zone, dest_owner).add(card_id);
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
        };
        apply_replacements(self, &mut event);
        if let ReplacementEvent::DamageToCard { amount: final_amount, .. } = event {
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
        let mut event = ReplacementEvent::DamageToPlayer {
            target,
            amount,
            source: None,
        };
        apply_replacements(self, &mut event);
        if let ReplacementEvent::DamageToPlayer { amount: final_amount, .. } = event {
            if final_amount > 0 {
                self.players[target.index()].deal_damage(final_amount);
            }
        }
    }

    /// Check and apply state-based actions. Returns true if any were applied.
    pub fn check_state_based_actions(&mut self) -> bool {
        let mut any_changes = false;

        // Check players with 0 or less life
        for pid in self.player_order.clone() {
            if self.player(pid).life <= 0 && self.player(pid).is_alive() {
                self.player_mut(pid).has_lost = true;
                any_changes = true;
            }
            // Check poison counters (10+ = lose)
            if self.player(pid).poison_counters >= 10 && self.player(pid).is_alive() {
                self.player_mut(pid).has_lost = true;
                any_changes = true;
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
                let lethal = card.lethal_damage() || (card.damage > 0 && card.has_deathtouch_damage);
                let should_die = zero_toughness || lethal;
                if should_die {
                    let owner = card.owner;
                    // CR 702.12: Indestructible prevents death from lethal damage and
                    // "destroy" effects, but NOT from toughness ≤ 0 (CR 704.5f vs 704.5g).
                    // This covers K:Indestructible from Forge card scripts (e.g. Darksteel Myr).
                    if lethal && !zero_toughness && self.cards[cid.index()].has_keyword("Indestructible") {
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
                        self.move_card(cid, final_dest, owner);
                        any_changes = true;
                    } else {
                        // Indestructible — destruction was replaced; creature stays.
                        // Damage is still marked but the creature does not die.
                    }
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
        let cards: Vec<CardId> = self
            .cards_in_zone(ZoneType::Battlefield, player)
            .to_vec();
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

        let all_card_ids: Vec<CardId> = (0..self.cards.len())
            .map(|i| CardId(i as u32))
            .collect();
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
            self.cards[host_id.index()].attachments.retain(|&a| a != aura_id);
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
