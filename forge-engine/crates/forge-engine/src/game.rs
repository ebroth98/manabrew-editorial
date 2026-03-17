use std::collections::{HashMap, VecDeque};

use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::card::CardInstance;
use crate::ids::{CardId, PlayerId};
use crate::phase::TurnState;
use crate::player::PlayerState;
use crate::spellability::MagicStack;
use crate::zone::{Zone, ZoneKey};

/// An extra turn entry — tracks who gets the turn and any modifications.
#[derive(Debug, Clone)]
pub struct ExtraTurn {
    pub player: PlayerId,
    /// If true, the untap step is skipped during this extra turn.
    pub skip_untap: bool,
}

/// Global registry of type lists loaded from `TypeLists.txt`.
///
/// Mirrors Java's `CardType.Constant.CREATURE_TYPES` etc., populated once by
/// `FModel.loadDynamicGamedata()` → `CardType.Helper.parseTypes()`.
///
/// Call [`TypeRegistry::load`] once at startup with the contents of
/// `TypeLists.txt`. All subsequent calls to [`TypeRegistry::creature_types`]
/// return the loaded data without any per-game copying.
pub struct TypeRegistry;

static CREATURE_TYPES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

impl TypeRegistry {
    /// Load creature types from the raw contents of `TypeLists.txt`.
    ///
    /// Parses the `[CreatureTypes]` section. Each line is either `TypeName` or
    /// `TypeName:PluralName`; only the singular (left of `:`) is kept.
    ///
    /// Mirrors Java's `FileSection.parseSections()` + `CardType.Helper.parseTypes()`.
    ///
    /// This must be called once before any game starts. Subsequent calls are
    /// silently ignored (first write wins).
    pub fn load(type_lists_content: &str) {
        let _ = CREATURE_TYPES.set(Self::parse_creature_types(type_lists_content));
    }

    /// Return the loaded creature types.
    ///
    /// # Panics
    /// Panics if [`TypeRegistry::load`] has not been called.
    pub fn creature_types() -> &'static [String] {
        CREATURE_TYPES.get().expect(
            "TypeRegistry: creature types not loaded. \
             Call TypeRegistry::load() with the contents of TypeLists.txt before starting a game.",
        )
    }

    fn parse_creature_types(content: &str) -> Vec<String> {
        let mut in_creature_section = false;
        let mut types = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_creature_section = &line[1..line.len() - 1] == "CreatureTypes";
                continue;
            }
            if in_creature_section {
                // "TypeName" or "TypeName:PluralName" — keep singular only
                let singular = line.split(':').next().unwrap_or(line);
                if !singular.is_empty() {
                    types.push(singular.to_string());
                }
            }
        }
        types
    }
}

/// The complete, serializable game state.
/// All game entities live here — nothing holds references, everything uses IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    // Arenas
    pub cards: Vec<CardInstance>,
    pub players: Vec<PlayerState>,

    // Zones: keyed by (ZoneType, PlayerId)
    #[serde(skip)]
    pub zones: HashMap<ZoneKey, Zone>,

    // The stack
    pub stack: MagicStack,

    // Turn/phase state
    pub turn: TurnState,

    // Player order (for turn sequence)
    pub player_order: Vec<PlayerId>,

    // Game over flag
    pub game_over: bool,
    pub winner: Option<PlayerId>,

    // Extra turns queue — players who get extra turns (issue #22, AddTurn effect).
    // After cleanup, the game pops from here instead of advancing to the next player.
    #[serde(skip)]
    pub extra_turns: VecDeque<ExtraTurn>,

    // Fog — prevent all combat damage this turn (issue #22, Fog effect).
    // Reset at end of turn cleanup.
    pub prevent_all_combat_damage: bool,

    // Monarch designation (issue #22, BecomeMonarch effect).
    pub monarch: Option<PlayerId>,

    // Initiative holder (issue #22, TakeInitiative effect).
    pub initiative_holder: Option<PlayerId>,

    // End turn requested — skip remaining phases, jump to cleanup (issue #22, EndTurn effect).
    pub end_turn_requested: bool,

    // End combat requested — skip remaining combat steps (issue #22, EndCombatPhase effect).
    pub end_combat_requested: bool,

    // Extra combat phases to insert after current combat (issue #22, AddPhase effect).
    pub extra_combat_phases: u32,

    // Next card ID counter
    next_card_id: u32,

    /// Monotonically increasing counter for zone-entry timestamps.
    /// Each time a card enters a zone, it gets the next value.
    /// Used to order same-player triggers by zone entry order,
    /// matching Java's `Zone.cardList` insertion order.
    next_zone_timestamp: u64,

    /// Periodic LKI snapshot of battlefield cards.
    /// Mirrors Java's `Game.lastStateBattlefield`.
    /// Updated by `copy_last_state()` at key game checkpoints.
    #[serde(skip)]
    pub last_state_battlefield: Vec<crate::lki::CardSnapshot>,
}

impl GameState {
    pub fn new(player_names: &[&str], starting_life: i32) -> Self {
        let mut players = Vec::new();
        let mut player_order = Vec::new();

        for (i, name) in player_names.iter().enumerate() {
            let pid = PlayerId(i as u32);
            players.push(PlayerState::new(pid, name.to_string(), starting_life));
            player_order.push(pid);
        }

        let mut zones = HashMap::new();
        let zone_types = [
            ZoneType::Hand,
            ZoneType::Library,
            ZoneType::Graveyard,
            ZoneType::Battlefield,
            ZoneType::Exile,
            ZoneType::Command,
            ZoneType::Stack,
        ];

        for &pid in &player_order {
            for &zt in &zone_types {
                let key = ZoneKey::new(zt, pid);
                zones.insert(key, Zone::new(zt, pid));
            }
        }

        GameState {
            cards: Vec::new(),
            players,
            zones,
            stack: MagicStack::new(),
            turn: TurnState::new(player_order[0], player_order.len() as u32),
            player_order,
            game_over: false,
            winner: None,
            extra_turns: VecDeque::new(),
            prevent_all_combat_damage: false,
            monarch: None,
            initiative_holder: None,
            end_turn_requested: false,
            end_combat_requested: false,
            extra_combat_phases: 0,
            next_card_id: 0,
            next_zone_timestamp: 0,
            last_state_battlefield: Vec::new(),
        }
    }

    /// Create a new card instance and return its ID. Does NOT place it in a zone.
    pub fn create_card(&mut self, mut card: CardInstance) -> CardId {
        let id = CardId(self.next_card_id);
        self.next_card_id += 1;
        card.id = id;
        self.cards.push(card);
        id
    }

    // --- Accessors ---

    pub fn card(&self, id: CardId) -> &CardInstance {
        &self.cards[id.index()]
    }

    pub fn card_mut(&mut self, id: CardId) -> &mut CardInstance {
        &mut self.cards[id.index()]
    }

    pub fn player(&self, id: PlayerId) -> &PlayerState {
        &self.players[id.index()]
    }

    pub fn player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        &mut self.players[id.index()]
    }

    pub fn zone(&self, zone_type: ZoneType, owner: PlayerId) -> &Zone {
        let key = ZoneKey::new(zone_type, owner);
        self.zones.get(&key).expect("Zone not found")
    }

    pub fn zone_mut(&mut self, zone_type: ZoneType, owner: PlayerId) -> &mut Zone {
        let key = ZoneKey::new(zone_type, owner);
        self.zones.get_mut(&key).expect("Zone not found")
    }

    pub fn active_player(&self) -> PlayerId {
        self.turn.active_player
    }

    pub fn next_player(&self, player: PlayerId) -> PlayerId {
        let current_idx = self
            .player_order
            .iter()
            .position(|&p| p == player)
            .unwrap_or(0);
        for i in 1..self.player_order.len() {
            let next_idx = (current_idx + i) % self.player_order.len();
            let next_pid = self.player_order[next_idx];
            if self.player(next_pid).is_alive() {
                return next_pid;
            }
        }
        player
    }

    pub fn opponent_of(&self, player: PlayerId) -> PlayerId {
        for &pid in &self.player_order {
            if pid != player && self.player(pid).is_alive() {
                return pid;
            }
        }
        player // no opponent found (shouldn't happen in normal games)
    }

    pub fn alive_players(&self) -> Vec<PlayerId> {
        self.player_order
            .iter()
            .filter(|&&pid| self.player(pid).is_alive())
            .copied()
            .collect()
    }

    /// Get all cards in a specific zone for a player.
    pub fn cards_in_zone(&self, zone_type: ZoneType, owner: PlayerId) -> &[CardId] {
        &self.zone(zone_type, owner).cards
    }

    /// Get all creatures on the battlefield for a player.
    pub fn creatures_on_battlefield(&self, player: PlayerId) -> Vec<CardId> {
        self.cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .filter(|&&cid| self.card(cid).is_creature())
            .copied()
            .collect()
    }

    /// Assign the next zone timestamp to a card, returning the value.
    /// Called whenever a card enters a new zone to track insertion order.
    pub fn assign_zone_timestamp(&mut self, card_id: CardId) -> u64 {
        let ts = self.next_zone_timestamp;
        self.next_zone_timestamp += 1;
        self.cards[card_id.index()].zone_timestamp = ts;
        ts
    }

    /// Get all lands on the battlefield for a player.
    pub fn lands_on_battlefield(&self, player: PlayerId) -> Vec<CardId> {
        self.cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .filter(|&&cid| self.card(cid).is_land())
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    #[test]
    fn create_game() {
        let game = GameState::new(&["Alice", "Bob"], 20);
        assert_eq!(game.players.len(), 2);
        assert_eq!(game.player(PlayerId(0)).name, "Alice");
        assert_eq!(game.player(PlayerId(1)).name, "Bob");
        assert_eq!(game.player(PlayerId(0)).life, 20);
    }

    #[test]
    fn create_card_and_zone() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let card = CardInstance::new(
            CardId(0),
            "Grizzly Bears".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let cid = game.create_card(card);
        game.zone_mut(ZoneType::Library, PlayerId(0)).add(cid);
        assert_eq!(game.zone(ZoneType::Library, PlayerId(0)).len(), 1);
    }

    #[test]
    fn opponent_lookup() {
        let game = GameState::new(&["Alice", "Bob"], 20);
        assert_eq!(game.opponent_of(PlayerId(0)), PlayerId(1));
        assert_eq!(game.opponent_of(PlayerId(1)), PlayerId(0));
    }

    #[test]
    fn lki_snapshot_captures_battlefield_state() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);

        // Create a 3/3 creature on the battlefield
        let mut card = CardInstance::new(
            CardId(0),
            "Grizzly Bears".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(3),
            Some(3),
            vec![],
            vec![],
        );
        card.zone = ZoneType::Battlefield;
        let cid = game.create_card(card);

        // Take LKI snapshot
        game.copy_last_state();

        // Verify snapshot captured the correct power/toughness
        let snapshot = game.get_lki_snapshot(cid).expect("snapshot should exist");
        assert_eq!(snapshot.power, 3);
        assert_eq!(snapshot.toughness, 3);
        assert_eq!(snapshot.card_name, "Grizzly Bears");

        // Move card to graveyard and verify snapshot still exists
        game.card_mut(cid).zone = ZoneType::Graveyard;
        let snapshot = game
            .get_lki_snapshot(cid)
            .expect("snapshot should still exist");
        assert_eq!(snapshot.power, 3);

        // Snapshot preserves stale entries for LKI (cards that left the battlefield).
        // This matches Java's behavior where LKI persists through resolution chains.
        game.copy_last_state();
        assert!(
            game.get_lki_snapshot(cid).is_some(),
            "stale LKI should persist"
        );
    }
}
