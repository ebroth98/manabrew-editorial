use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Game state mutation methods — moving cards, dealing damage, state-based actions.
impl GameState {
    /// Move a card from its current zone to a new zone.
    pub fn move_card(&mut self, card_id: CardId, dest_zone: ZoneType, dest_owner: PlayerId) {
        let card = &self.cards[card_id.index()];
        let src_zone = card.zone;
        let src_owner = card.controller;

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
            }
            ZoneType::Graveyard | ZoneType::Hand | ZoneType::Exile | ZoneType::Library => {
                // Reset battlefield state when leaving
                let card = &mut self.cards[card_id.index()];
                card.tapped = false;
                card.damage = 0;
                card.power_modifier = 0;
                card.toughness_modifier = 0;
                card.summoning_sick = true;
                card.controller = card.owner; // controller resets to owner
            }
            _ => {}
        }

        // Add to destination zone
        self.zone_mut(dest_zone, dest_owner).add(card_id);
    }

    /// Deal damage to a card (creature).
    pub fn deal_damage_to_card(&mut self, target: CardId, amount: i32) {
        if amount > 0 {
            self.cards[target.index()].damage += amount;
        }
    }

    /// Deal damage to a player.
    pub fn deal_damage_to_player(&mut self, target: PlayerId, amount: i32) {
        if amount > 0 {
            self.players[target.index()].deal_damage(amount);
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
                let should_die = card.toughness() <= 0 || card.lethal_damage();
                if should_die {
                    let owner = card.owner;
                    self.move_card(cid, ZoneType::Graveyard, owner);
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
        let cards: Vec<CardId> = self
            .cards_in_zone(ZoneType::Battlefield, player)
            .to_vec();
        for cid in cards {
            self.cards[cid.index()].tapped = false;
        }
    }

    /// Draw a card for a player. Returns the drawn card ID, or None if library empty.
    pub fn draw_card(&mut self, player: PlayerId) -> Option<CardId> {
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
