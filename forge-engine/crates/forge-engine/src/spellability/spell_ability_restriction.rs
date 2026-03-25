//! Activation restrictions for spell abilities.
//!
//! Mirrors Java's `SpellAbilityRestriction.java` — determines whether a
//! spell ability can be legally activated given the current game state.

use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::Params;

use super::spell_ability_variables::SpellAbilityVariables;

/// Activation restrictions for a spell ability.
/// Mirrors Java's `SpellAbilityRestriction` — wraps `SpellAbilityVariables`
/// and checks game state conditions to determine if activation is legal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellAbilityRestriction {
    pub variables: SpellAbilityVariables,
}

impl Default for SpellAbilityRestriction {
    fn default() -> Self {
        Self {
            variables: SpellAbilityVariables::new(),
        }
    }
}

impl SpellAbilityRestriction {
    /// Create a new restriction with default variables.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse activation restrictions from ability params.
    /// Mirrors Java's `SpellAbilityRestriction.setRestrictions(SpellAbility)`.
    pub fn set_restrictions(&mut self, params: &Params) {
        // Parse activation zone
        if let Some(zone_str) = params.get("ActivationZone") {
            if let Some(zone) = parse_zone(zone_str) {
                self.variables.set_zone(zone);
            }
        }

        // Parse phase restrictions
        if let Some(phases_str) = params.get("ActivationPhases") {
            for phase_name in phases_str.split(',') {
                if let Some(phase) = PhaseType::from_script_name(phase_name.trim()) {
                    self.variables.add_phase(phase);
                }
            }
        }

        // Parse sorcery speed restriction
        if params.is_true("SorcerySpeed") {
            self.variables.set_sorcery_speed(true);
        }

        // Parse instant speed
        if params.is_true("InstantSpeed") {
            self.variables.set_instant_speed(true);
        }

        // Parse activator
        if let Some(activator) = params.get("Activator") {
            self.variables.set_activator(activator.to_string());
        }

        // Parse turn restrictions
        if params.is_true("PlayerTurn") {
            self.variables.set_player_turn(true);
        }
        if params.is_true("OpponentTurn") {
            self.variables.set_opponent_turn(true);
        }

        // Parse activation limits
        if let Some(limit) = params.get("ActivationLimit") {
            self.variables.set_limit_to_check(Some(limit.to_string()));
        }
        if let Some(game_limit) = params.get("GameActivationLimit") {
            self.variables
                .set_game_limit_to_check(Some(game_limit.to_string()));
        }

        // Parse condition flags
        if params.is_true("Threshold") {
            self.variables.set_threshold(true);
        }
        if params.is_true("Metalcraft") {
            self.variables.set_metalcraft(true);
        }
        if params.is_true("Delirium") {
            self.variables.set_delirium(true);
        }
        if params.is_true("Hellbent") {
            self.variables.set_hellbent(true);
        }
        if params.is_true("Revolt") {
            self.variables.set_revolt(true);
        }
        if params.is_true("Desert") {
            self.variables.set_desert(true);
        }
        if params.is_true("Blessing") {
            self.variables.set_blessing(true);
        }
        if params.is_true("Solved") {
            self.variables.set_solved(true);
        }

        // Parse presence check
        if let Some(present) = params.get("IsPresent") {
            self.variables.set_is_present(Some(present.to_string()));
        }
        if let Some(compare) = params.get("PresentCompare") {
            self.variables
                .set_present_compare(Some(compare.to_string()));
        }
        if let Some(zone_str) = params.get("PresentZone") {
            if let Some(zone) = parse_zone(zone_str) {
                self.variables.set_present_zone(zone);
            }
        }
        if let Some(defined) = params.get("PresentDefined") {
            self.variables
                .set_present_defined(Some(defined.to_string()));
        }

        // Parse cards in hand requirement
        if let Some(count_str) = params.get("ActivateCardsInHand") {
            if let Ok(count) = count_str.parse::<i32>() {
                self.variables.set_cards_in_hand(count);
            }
        }
    }

    /// Check if this spell ability can be played given the current game state.
    /// Mirrors Java's `SpellAbilityRestriction.canPlay(Card, SpellAbility)`.
    pub fn can_play(&self, game: &GameState, card_id: CardId, player: PlayerId) -> bool {
        let card = game.card(card_id);

        // Check zone restriction
        let card_zone = card.zone;
        if card_zone != self.variables.zone() {
            return false;
        }

        // Check phase restriction
        let phases = self.variables.phases();
        if !phases.is_empty() && !phases.contains(&game.turn.phase) {
            return false;
        }

        // Check sorcery speed: must be a main phase and the stack must be empty
        if self.variables.sorcery_speed() {
            let is_main = game.turn.phase.is_main();
            let stack_empty = game.stack.is_empty();
            let is_active = game.turn.active_player == player;
            if !is_main || !stack_empty || !is_active {
                return false;
            }
        }

        // Check turn restrictions
        let is_players_turn = game.turn.active_player == player;
        if self.variables.player_turn() && !is_players_turn {
            return false;
        }
        if self.variables.opponent_turn() && is_players_turn {
            return false;
        }

        // Check cards in hand requirement
        let required = self.variables.cards_in_hand();
        if required >= 0 {
            let hand_count = game.cards_in_zone(ZoneType::Hand, player).len() as i32;
            if hand_count < required {
                return false;
            }
        }

        // Check hellbent (no cards in hand)
        if self.variables.hellbent() {
            let hand_count = game.cards_in_zone(ZoneType::Hand, player).len();
            if hand_count > 0 {
                return false;
            }
        }

        // Check threshold (7+ cards in graveyard)
        if self.variables.threshold() {
            let gy_count = game.cards_in_zone(ZoneType::Graveyard, player).len();
            if gy_count < 7 {
                return false;
            }
        }

        // Check metalcraft (3+ artifacts on battlefield)
        if self.variables.metalcraft() {
            let artifact_count = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .iter()
                .filter(|&&cid| game.card(cid).type_line.is_artifact())
                .count();
            if artifact_count < 3 {
                return false;
            }
        }

        // Check delirium (4+ card types in graveyard)
        if self.variables.delirium() {
            let mut types = std::collections::HashSet::new();
            for &cid in game.cards_in_zone(ZoneType::Graveyard, player) {
                let c = game.card(cid);
                if c.is_creature() {
                    types.insert("creature");
                }
                if c.is_land() {
                    types.insert("land");
                }
                if c.type_line.is_artifact() {
                    types.insert("artifact");
                }
                if c.type_line.is_enchantment() {
                    types.insert("enchantment");
                }
                if c.type_line.is_instant() {
                    types.insert("instant");
                }
                if c.type_line.is_sorcery() {
                    types.insert("sorcery");
                }
                if c.type_line.is_planeswalker() {
                    types.insert("planeswalker");
                }
            }
            if types.len() < 4 {
                return false;
            }
        }

        true
    }

    /// Check zone restrictions only.
    /// Mirrors Java's `SpellAbilityRestriction.checkZoneRestrictions(Card, SpellAbility)`.
    pub fn check_zone_restrictions(&self, game: &GameState, card_id: CardId) -> bool {
        let card = game.card(card_id);
        card.zone == self.variables.zone()
    }

    /// Check timing restrictions (phase, sorcery speed, etc.).
    /// Mirrors Java's `SpellAbilityRestriction.checkTimingRestrictions(Card, SpellAbility)`.
    pub fn check_timing_restrictions(&self, game: &GameState, player: PlayerId) -> bool {
        let phases = self.variables.phases();
        if !phases.is_empty() && !phases.contains(&game.turn.phase) {
            return false;
        }
        if self.variables.sorcery_speed() {
            let is_main = game.turn.phase.is_main();
            let stack_empty = game.stack.is_empty();
            let is_active = game.turn.active_player == player;
            if !is_main || !stack_empty || !is_active {
                return false;
            }
        }
        let is_players_turn = game.turn.active_player == player;
        if self.variables.player_turn() && !is_players_turn {
            return false;
        }
        if self.variables.opponent_turn() && is_players_turn {
            return false;
        }
        true
    }

    /// Check activator restrictions.
    /// Mirrors Java's `SpellAbilityRestriction.checkActivatorRestrictions(Card, SpellAbility)`.
    pub fn check_activator_restrictions(&self, game: &GameState, player: PlayerId) -> bool {
        let activator = self.variables.activator();
        if activator == "You" {
            return true;
        }
        if activator == "Opponent" {
            return game.turn.active_player != player;
        }
        true
    }

    /// Check other restrictions (threshold, metalcraft, etc.).
    /// Mirrors Java's `SpellAbilityRestriction.checkOtherRestrictions(Card, SpellAbility)`.
    pub fn check_other_restrictions(&self, game: &GameState, player: PlayerId) -> bool {
        if self.variables.hellbent() {
            let hand_count = game.cards_in_zone(ZoneType::Hand, player).len();
            if hand_count > 0 {
                return false;
            }
        }
        if self.variables.threshold() {
            let gy_count = game.cards_in_zone(ZoneType::Graveyard, player).len();
            if gy_count < 7 {
                return false;
            }
        }
        if self.variables.metalcraft() {
            let artifact_count = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .iter()
                .filter(|&&cid| game.card(cid).type_line.is_artifact())
                .count();
            if artifact_count < 3 {
                return false;
            }
        }
        true
    }
}

/// Parse a zone string into a ZoneType.
fn parse_zone(s: &str) -> Option<ZoneType> {
    match s.to_lowercase().as_str() {
        "battlefield" => Some(ZoneType::Battlefield),
        "hand" => Some(ZoneType::Hand),
        "graveyard" => Some(ZoneType::Graveyard),
        "library" => Some(ZoneType::Library),
        "exile" => Some(ZoneType::Exile),
        "command" => Some(ZoneType::Command),
        _ => None,
    }
}
