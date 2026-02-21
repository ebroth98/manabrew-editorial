use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Tracks combat state for the current combat phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatState {
    /// Attacking player.
    pub attacking_player: Option<PlayerId>,
    /// Defending player.
    pub defending_player: Option<PlayerId>,
    /// (attacker CardId, defending player)
    pub attackers: Vec<(CardId, PlayerId)>,
    /// (blocker CardId, attacker CardId)
    pub blockers: Vec<(CardId, CardId)>,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.attacking_player = None;
        self.defending_player = None;
        self.attackers.clear();
        self.blockers.clear();
    }

    pub fn declare_attacker(&mut self, attacker: CardId, defending: PlayerId) {
        self.attackers.push((attacker, defending));
    }

    pub fn declare_blocker(&mut self, blocker: CardId, attacker: CardId) {
        self.blockers.push((blocker, attacker));
    }

    pub fn is_attacking(&self, card: CardId) -> bool {
        self.attackers.iter().any(|(a, _)| *a == card)
    }

    pub fn is_blocked(&self, attacker: CardId) -> bool {
        self.blockers.iter().any(|(_, a)| *a == attacker)
    }

    pub fn get_blockers_for(&self, attacker: CardId) -> Vec<CardId> {
        self.blockers
            .iter()
            .filter(|(_, a)| *a == attacker)
            .map(|(b, _)| *b)
            .collect()
    }

    pub fn has_attackers(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// Check if any creature in combat has first strike or double strike.
    pub fn has_first_strikers(&self, game: &GameState) -> bool {
        for &(attacker_id, _) in &self.attackers {
            if game.card(attacker_id).zone != ZoneType::Battlefield {
                continue;
            }
            let card = game.card(attacker_id);
            if card.has_first_strike() || card.has_double_strike() {
                return true;
            }
        }
        for &(blocker_id, _) in &self.blockers {
            if game.card(blocker_id).zone != ZoneType::Battlefield {
                continue;
            }
            let card = game.card(blocker_id);
            if card.has_first_strike() || card.has_double_strike() {
                return true;
            }
        }
        false
    }

    /// Resolve one step of combat damage.
    /// If `first_strike_only` is true, only first-strike and double-strike creatures deal damage.
    /// If false, only non-first-strike and double-strike creatures deal damage.
    pub fn resolve_damage_step(&self, game: &mut GameState, first_strike_only: bool) {
        for (attacker_id, defending_player) in self.attackers.clone() {
            // Check attacker is still alive
            if game.card(attacker_id).zone != ZoneType::Battlefield {
                continue;
            }

            let attacker = game.card(attacker_id);
            let attacker_has_fs = attacker.has_first_strike();
            let attacker_has_ds = attacker.has_double_strike();
            let attacker_has_trample = attacker.has_trample();
            let attacker_has_deathtouch = attacker.has_deathtouch();
            let attacker_has_lifelink = attacker.has_lifelink();
            let attacker_controller = attacker.controller;

            // Determine if this attacker deals damage in this step
            let deals_damage = if first_strike_only {
                attacker_has_fs || attacker_has_ds
            } else {
                // Regular damage step: creatures without first strike, plus double strike
                !attacker_has_fs || attacker_has_ds
            };

            if !deals_damage {
                continue;
            }

            let attacker_power = game.card(attacker_id).power();
            if attacker_power <= 0 {
                continue;
            }

            let blockers = self.get_blockers_for(attacker_id);

            if blockers.is_empty() {
                // Unblocked — damage goes to defending player
                deal_combat_damage_to_player(
                    game,
                    defending_player,
                    attacker_power,
                    attacker_has_lifelink,
                    attacker_controller,
                );
                // Track commander damage
                if game.card(attacker_id).is_commander {
                    *game
                        .player_mut(defending_player)
                        .commander_damage_received
                        .entry(attacker_id.0)
                        .or_insert(0) += attacker_power;
                }
            } else {
                // Blocked — mutual damage
                let mut remaining_damage = attacker_power;

                for &blocker_id in &blockers {
                    if remaining_damage <= 0 {
                        break;
                    }
                    // Check blocker is still alive
                    if game.card(blocker_id).zone != ZoneType::Battlefield {
                        continue;
                    }

                    let blocker_toughness = game.card(blocker_id).toughness();
                    let blocker_damage = game.card(blocker_id).damage;
                    let remaining_toughness = blocker_toughness - blocker_damage;

                    // Deathtouch: only 1 damage needed to be lethal
                    let damage_to_blocker = if attacker_has_deathtouch {
                        1.min(remaining_damage)
                    } else {
                        remaining_damage.min(remaining_toughness.max(0))
                    };

                    if damage_to_blocker > 0 {
                        deal_combat_damage_to_card(
                            game,
                            blocker_id,
                            damage_to_blocker,
                            attacker_has_deathtouch,
                            attacker_has_lifelink,
                            attacker_controller,
                        );
                        remaining_damage -= damage_to_blocker;
                    }

                    // Blocker damages attacker (only in the step it should deal damage)
                    let blocker_card = game.card(blocker_id);
                    let blocker_has_fs = blocker_card.has_first_strike();
                    let blocker_has_ds = blocker_card.has_double_strike();
                    let blocker_has_deathtouch = blocker_card.has_deathtouch();
                    let blocker_has_lifelink = blocker_card.has_lifelink();
                    let blocker_controller = blocker_card.controller;

                    let blocker_deals = if first_strike_only {
                        blocker_has_fs || blocker_has_ds
                    } else {
                        !blocker_has_fs || blocker_has_ds
                    };

                    if blocker_deals {
                        let blocker_power = game.card(blocker_id).power();
                        if blocker_power > 0 {
                            deal_combat_damage_to_card(
                                game,
                                attacker_id,
                                blocker_power,
                                blocker_has_deathtouch,
                                blocker_has_lifelink,
                                blocker_controller,
                            );
                        }
                    }
                }

                // Trample: remaining damage goes to defending player
                if attacker_has_trample && remaining_damage > 0 {
                    deal_combat_damage_to_player(
                        game,
                        defending_player,
                        remaining_damage,
                        attacker_has_lifelink,
                        attacker_controller,
                    );
                    // Track commander damage from trample
                    if game.card(attacker_id).is_commander {
                        *game
                            .player_mut(defending_player)
                            .commander_damage_received
                            .entry(attacker_id.0)
                            .or_insert(0) += remaining_damage;
                    }
                }
            }
        }
    }
}

// ── Combat helper functions ─────────────────────────────────────────

/// Get available attackers: untapped creatures that can attack.
pub fn get_available_attackers(game: &GameState, player: PlayerId) -> Vec<CardId> {
    game.creatures_on_battlefield(player)
        .into_iter()
        .filter(|&cid| game.card(cid).can_attack())
        .collect()
}

/// Get available blockers: untapped creatures that can block.
pub fn get_available_blockers(game: &GameState, player: PlayerId) -> Vec<CardId> {
    game.creatures_on_battlefield(player)
        .into_iter()
        .filter(|&cid| game.card(cid).can_block())
        .collect()
}

/// Filter blockers to only those that can legally block at least one attacker.
/// A creature without flying or reach cannot block a flier.
pub fn filter_legal_blockers(
    game: &GameState,
    attackers: &[CardId],
    blockers: &[CardId],
) -> Vec<CardId> {
    blockers
        .iter()
        .filter(|&&blocker_id| {
            let blocker = game.card(blocker_id);
            // A blocker is legal if it can block at least one attacker
            attackers.iter().any(|&attacker_id| {
                let attacker = game.card(attacker_id);
                if attacker.has_flying() {
                    blocker.has_flying() || blocker.has_reach()
                } else {
                    true
                }
            })
        })
        .copied()
        .collect()
}

/// Deal combat damage to a player, handling lifelink.
fn deal_combat_damage_to_player(
    game: &mut GameState,
    target: PlayerId,
    amount: i32,
    lifelink: bool,
    source_controller: PlayerId,
) {
    if amount > 0 {
        game.deal_damage_to_player(target, amount);
        if lifelink {
            game.player_mut(source_controller).gain_life(amount);
        }
    }
}

/// Deal combat damage to a card, handling deathtouch and lifelink.
fn deal_combat_damage_to_card(
    game: &mut GameState,
    target: CardId,
    amount: i32,
    deathtouch: bool,
    lifelink: bool,
    source_controller: PlayerId,
) {
    if amount > 0 {
        game.deal_damage_to_card(target, amount);
        if deathtouch {
            game.card_mut(target).has_deathtouch_damage = true;
        }
        if lifelink {
            game.player_mut(source_controller).gain_life(amount);
        }
    }
}
