use std::collections::HashSet;

use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// A combat damage event returned from resolve_damage_step.
/// Used to fire DamageDone and LifeGained triggers from game_loop.rs.
#[derive(Debug, Clone)]
pub struct CombatDamageEvent {
    pub source: CardId,
    pub target_player: Option<PlayerId>,
    pub target_card: Option<CardId>,
    pub amount: i32,
    pub is_combat: bool,
    pub lifelink_player: Option<PlayerId>,
    pub lifelink_amount: i32,
}

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

    /// Clear combat state, including the `attacking_player` flag on each attacker card.
    pub fn clear_with_cards(&mut self, cards: &mut [crate::card::CardInstance]) {
        for &(attacker_id, _) in &self.attackers {
            cards[attacker_id.index()].attacking_player = None;
        }
        self.clear();
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
    /// Returns a Vec of CombatDamageEvents so the caller can fire triggers.
    pub fn resolve_damage_step(
        &self,
        game: &mut GameState,
        first_strike_only: bool,
        as_unblocked_choices: &HashSet<CardId>,
    ) -> Vec<CombatDamageEvent> {
        // Fog effect: skip all combat damage this turn (issue #22).
        if game.prevent_all_combat_damage {
            return Vec::new();
        }

        let mut events = Vec::new();

        for (attacker_id, defending_player) in self.attackers.clone() {
            // Check attacker is still alive
            if game.card(attacker_id).zone != ZoneType::Battlefield {
                continue;
            }

            let attacker = game.card(attacker_id);
            if crate::staticability::static_ability_assign_no_combat_damage::assign_no_combat_damage(
                &game.cards,
                attacker,
            ) {
                continue;
            }
            let attacker_has_fs = attacker.has_first_strike();
            let attacker_has_ds = attacker.has_double_strike();
            let _attacker_has_trample = attacker.has_trample();
            let attacker_has_deathtouch = attacker.has_deathtouch();
            let attacker_has_lifelink = attacker.has_lifelink();
            let attacker_has_infect = attacker.has_infect()
                || crate::staticability::static_ability_infect_damage::is_infect_damage(
                    game,
                    &game.cards,
                    defending_player,
                    attacker.controller,
                );
            let attacker_has_wither = attacker.has_wither()
                || crate::staticability::static_ability_wither_damage::is_wither_damage(
                    &game.cards,
                    attacker,
                );
            let attacker_toxic_count = attacker.get_toxic_count();
            let attacker_controller = attacker.controller;

            // Determine if this attacker deals damage in this step
            let attacker_deals_damage = if first_strike_only {
                attacker_has_fs || attacker_has_ds
            } else {
                // Regular damage step: creatures without first strike, plus double strike
                !attacker_has_fs || attacker_has_ds
            };

            let attacker_power = if crate::staticability::static_ability_combat_damage_toughness::combat_damage_uses_toughness(
                &game.cards,
                game.card(attacker_id),
            ) {
                game.card(attacker_id).toughness()
            } else {
                game.card(attacker_id).power()
            };

            let attacker_card = game.card(attacker_id);
            let assign_as_unblocked =
                crate::staticability::static_ability_assign_combat_damage_as_unblocked::has_mandatory_assign_as_unblocked(
                    &game.cards,
                    attacker_card,
                )
                    || crate::staticability::static_ability_assign_combat_damage_as_unblocked::assign_as_unblocked(
                        &game.cards,
                        attacker_card,
                        as_unblocked_choices.contains(&attacker_id),
                    );

            let blockers = if assign_as_unblocked {
                Vec::new()
            } else {
                self.get_blockers_for(attacker_id)
            };

            if blockers.is_empty() {
                // Unblocked — damage goes to defending player
                if !attacker_deals_damage || attacker_power <= 0 {
                    continue;
                }
                deal_combat_damage_to_player(
                    game,
                    defending_player,
                    attacker_power,
                    attacker_has_lifelink,
                    attacker_controller,
                    attacker_has_infect,
                    attacker_toxic_count,
                );
                events.push(CombatDamageEvent {
                    source: attacker_id,
                    target_player: Some(defending_player),
                    target_card: None,
                    amount: attacker_power,
                    is_combat: true,
                    lifelink_player: if attacker_has_lifelink {
                        Some(attacker_controller)
                    } else {
                        None
                    },
                    lifelink_amount: if attacker_has_lifelink {
                        attacker_power
                    } else {
                        0
                    },
                });
                // Track commander damage
                if game.card(attacker_id).is_commander {
                    *game
                        .player_mut(defending_player)
                        .commander_damage_received
                        .entry(attacker_id.0)
                        .or_insert(0) += attacker_power;
                }
            } else {
                // Blocked — mutual damage.
                // The attacker may not deal damage this step (e.g. no first strike during
                // first-strike step), but blockers with the right timing still deal damage
                // back to the attacker.
                let remaining_damage = if attacker_deals_damage && attacker_power > 0 {
                    attacker_power
                } else {
                    0
                };

                // --- Compute damage assignment for each blocker (Java-parity) ---
                // Java's DeterministicController assigns lethal to each blocker
                // in order, then puts all remaining damage on the last blocker.
                // We pre-compute the total per blocker so each gets a single
                // damage event (critical for DamageDoneOnce triggers like Raptor
                // Hatchling that should only fire once per blocker).
                let alive_blockers: Vec<CardId> = blockers
                    .iter()
                    .copied()
                    .filter(|&bid| game.card(bid).zone == ZoneType::Battlefield)
                    .collect();
                let mut damage_assignments: Vec<(CardId, i32)> = Vec::new();
                {
                    let mut dmg_left = remaining_damage;
                    for (idx, &blocker_id) in alive_blockers.iter().enumerate() {
                        if dmg_left <= 0 {
                            break;
                        }
                        let is_last = idx == alive_blockers.len() - 1;
                        let attacker_prevented = crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
                            &game.cards,
                            game.card(blocker_id),
                            game.card(attacker_id),
                        );
                        if attacker_prevented {
                            continue;
                        }

                        let damage_to_blocker = if is_last {
                            // Last blocker gets ALL remaining damage (matches Java).
                            dmg_left
                        } else if attacker_has_deathtouch {
                            1.min(dmg_left)
                        } else {
                            let blocker_toughness = game.card(blocker_id).toughness();
                            let blocker_damage = game.card(blocker_id).damage;
                            let remaining_toughness = blocker_toughness - blocker_damage;
                            dmg_left.min(remaining_toughness.max(0))
                        };

                        if damage_to_blocker > 0 {
                            damage_assignments.push((blocker_id, damage_to_blocker));
                            dmg_left -= damage_to_blocker;
                        }
                    }
                }

                // Deal the pre-computed damage to each blocker in a single event.
                for &(blocker_id, damage_to_blocker) in &damage_assignments {
                    deal_combat_damage_to_card(
                        game,
                        attacker_id,
                        blocker_id,
                        damage_to_blocker,
                        attacker_has_deathtouch,
                        attacker_has_lifelink,
                        attacker_controller,
                        attacker_has_wither || attacker_has_infect,
                    );
                    events.push(CombatDamageEvent {
                        source: attacker_id,
                        target_player: None,
                        target_card: Some(blocker_id),
                        amount: damage_to_blocker,
                        is_combat: true,
                        lifelink_player: if attacker_has_lifelink {
                            Some(attacker_controller)
                        } else {
                            None
                        },
                        lifelink_amount: if attacker_has_lifelink {
                            damage_to_blocker
                        } else {
                            0
                        },
                    });
                }

                for &blocker_id in &blockers {
                    // Check blocker is still alive
                    if game.card(blocker_id).zone != ZoneType::Battlefield {
                        continue;
                    }

                    // --- Blocker damages attacker (independent of whether attacker deals damage) ---
                    let blocker_card = game.card(blocker_id);
                    if crate::staticability::static_ability_assign_no_combat_damage::assign_no_combat_damage(
                        &game.cards,
                        blocker_card,
                    ) {
                        continue;
                    }
                    let blocker_has_fs = blocker_card.has_first_strike();
                    let blocker_has_ds = blocker_card.has_double_strike();
                    let blocker_has_deathtouch = blocker_card.has_deathtouch();
                    let blocker_has_lifelink = blocker_card.has_lifelink();
                    let blocker_has_infect = blocker_card.has_infect()
                        || crate::staticability::static_ability_infect_damage::is_infect_damage(
                            game,
                            &game.cards,
                            game.card(attacker_id).controller,
                            blocker_card.controller,
                        );
                    let blocker_has_wither = blocker_card.has_wither()
                        || crate::staticability::static_ability_wither_damage::is_wither_damage(
                            &game.cards,
                            blocker_card,
                        );
                    let blocker_controller = blocker_card.controller;

                    let blocker_deals = if first_strike_only {
                        blocker_has_fs || blocker_has_ds
                    } else {
                        !blocker_has_fs || blocker_has_ds
                    };

                    if blocker_deals {
                        // Protection damage prevention — see note above.
                        if crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
                            &game.cards,
                            game.card(attacker_id),
                            game.card(blocker_id),
                        ) {
                            continue;
                        }

                        let blocker_power = if crate::staticability::static_ability_combat_damage_toughness::combat_damage_uses_toughness(
                            &game.cards,
                            game.card(blocker_id),
                        ) {
                            game.card(blocker_id).toughness()
                        } else {
                            game.card(blocker_id).power()
                        };
                        if blocker_power > 0 {
                            deal_combat_damage_to_card(
                                game,
                                blocker_id,
                                attacker_id,
                                blocker_power,
                                blocker_has_deathtouch,
                                blocker_has_lifelink,
                                blocker_controller,
                                blocker_has_wither || blocker_has_infect,
                            );
                            events.push(CombatDamageEvent {
                                source: blocker_id,
                                target_player: None,
                                target_card: Some(attacker_id),
                                amount: blocker_power,
                                is_combat: true,
                                lifelink_player: if blocker_has_lifelink {
                                    Some(blocker_controller)
                                } else {
                                    None
                                },
                                lifelink_amount: if blocker_has_lifelink {
                                    blocker_power
                                } else {
                                    0
                                },
                            });
                        }
                    }
                }

                // Note: excess damage was already assigned to the last blocker
                // above (matching Java's DeterministicController behavior).
            }
        }

        events
    }
}

// ── Combat helper functions ─────────────────────────────────────────

/// Get available attackers: untapped creatures that can attack.
pub fn get_available_attackers(game: &GameState, player: PlayerId) -> Vec<CardId> {
    let defending = game.opponent_of(player);
    game.creatures_on_battlefield(player)
        .into_iter()
        .filter(|&cid| {
            let card = game.card(cid);
            if card.can_attack() {
                return true;
            }
            card.is_creature()
                && !card.tapped
                && !card.cant_attack_static
                && !card.detained
                && (card.has_haste() || !card.summoning_sick)
                && card.zone == ZoneType::Battlefield
                && card.has_defender()
                && crate::staticability::static_ability_can_attack_defender::can_attack_defender(
                    &game.cards, card, defending,
                )
        })
        .collect()
}

/// Get available blockers: untapped creatures that can block.
pub fn get_available_blockers(game: &GameState, player: PlayerId) -> Vec<CardId> {
    game.creatures_on_battlefield(player)
        .into_iter()
        .filter(|&cid| game.card(cid).can_block())
        .collect()
}

/// Check if a specific blocker can legally block a specific attacker.
/// Mirrors Java's CombatUtil.canBlock().
pub fn can_creature_block(game: &GameState, blocker_id: CardId, attacker_id: CardId) -> bool {
    let attacker = game.card(attacker_id);
    let blocker = game.card(blocker_id);

    // Blockers must satisfy baseline legality (untapped, creature, not detained, etc.).
    if !blocker.can_block() {
        return false;
    }

    // Flying: only blocked by flying or reach
    if attacker.has_flying() && !blocker.has_flying() && !blocker.has_reach() {
        return false;
    }
    // Fear: only blocked by artifact or black creatures
    if attacker.has_fear() && !blocker.type_line.is_artifact() && !blocker.color.has_black() {
        return false;
    }
    // Intimidate: only blocked by artifact or creatures sharing a color
    if attacker.has_intimidate()
        && !blocker.type_line.is_artifact()
        && !blocker.color.shares_color_with(attacker.color)
    {
        return false;
    }
    // Shadow: shadow only blocked by shadow, non-shadow not blocked by shadow
    if attacker.has_shadow() != blocker.has_shadow() {
        return false;
    }
    // Horsemanship: only blocked by horsemanship
    if attacker.has_horsemanship() && !blocker.has_horsemanship() {
        return false;
    }
    // Skulk: can't be blocked by creatures with greater power
    if attacker.has_skulk() && blocker.power() > attacker.power() {
        return false;
    }
    // Protection: can't be blocked by matching creatures
    if attacker.is_protected_from(blocker) {
        return false;
    }
    true
}

/// Filter blockers to only those that can legally block at least one attacker.
/// Accounts for Flying, Fear, Intimidate, Shadow, Horsemanship, Skulk, Protection.
pub fn filter_legal_blockers(
    game: &GameState,
    attackers: &[CardId],
    blockers: &[CardId],
) -> Vec<CardId> {
    blockers
        .iter()
        .filter(|&&blocker_id| {
            // A blocker is legal if it can block at least one attacker
            attackers
                .iter()
                .any(|&attacker_id| can_creature_block(game, blocker_id, attacker_id))
        })
        .copied()
        .collect()
}

/// Deal combat damage to a player, handling lifelink, Infect, and Toxic.
fn deal_combat_damage_to_player(
    game: &mut GameState,
    target: PlayerId,
    amount: i32,
    lifelink: bool,
    source_controller: PlayerId,
    source_has_infect: bool,
    source_toxic_count: Option<i32>,
) {
    if amount > 0 {
        if source_has_infect {
            // Infect: deal damage as poison counters instead of life loss
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &game.cards,
                target,
                &crate::card::CounterType::Poison,
            ) {
                game.player_mut(target).poison_counters += amount;
            }
        } else {
            game.deal_damage_to_player(target, amount);
        }
        // Toxic: add poison counters in addition to normal damage
        if let Some(toxic) = source_toxic_count {
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &game.cards,
                target,
                &crate::card::CounterType::Poison,
            ) {
                game.player_mut(target).poison_counters += toxic;
            }
        }
        if lifelink {
            game.player_mut(source_controller).gain_life(amount);
        }
    }
}

/// Deal combat damage to a card, handling deathtouch, lifelink, Infect/Wither.
fn deal_combat_damage_to_card(
    game: &mut GameState,
    source: CardId,
    target: CardId,
    amount: i32,
    deathtouch: bool,
    lifelink: bool,
    source_controller: PlayerId,
    source_has_wither_or_infect: bool,
) {
    if amount > 0 {
        // Track damage source for DamagedBy trigger filters (Sengir Vampire, etc.)
        if !game.card(target).damage_sources_this_turn.contains(&source) {
            game.card_mut(target).damage_sources_this_turn.push(source);
        }
        if source_has_wither_or_infect {
            // Wither/Infect: damage to creatures as -1/-1 counters instead
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                &game.cards,
                game.card(target),
                &crate::card::CounterType::M1M1,
            ) {
                game.card_mut(target)
                    .add_counter(&crate::card::CounterType::M1M1, amount);
            }
        } else {
            game.deal_damage_to_card(target, amount);
        }
        if deathtouch {
            game.card_mut(target).has_deathtouch_damage = true;
        }
        if lifelink {
            game.player_mut(source_controller).gain_life(amount);
        }
    }
}
