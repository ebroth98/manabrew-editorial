pub mod attack_cost;
pub mod attack_requirement;
pub mod attack_restriction;
pub mod block_cost;

use std::collections::{HashMap, HashSet};

use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::agent::PlayerAgent;
use crate::card::valid_filter;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::staticability::static_ability::StaticMode;

/// Identifies the target of an attack: a player or a permanent (planeswalker/battle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DefenderId {
    Player(PlayerId),
    Permanent(CardId),
}

impl DefenderId {
    /// Returns the PlayerId if this is a player defender, or the controller
    /// of the permanent if it's a planeswalker/battle.
    pub fn controlling_player(&self, game: &GameState) -> PlayerId {
        match self {
            DefenderId::Player(pid) => *pid,
            DefenderId::Permanent(cid) => game.card(*cid).controller,
        }
    }

    /// Returns the PlayerId if this is a player defender.
    pub fn as_player(&self) -> Option<PlayerId> {
        match self {
            DefenderId::Player(pid) => Some(*pid),
            DefenderId::Permanent(_) => None,
        }
    }
}

/// Last-known-information snapshot for a creature that left combat.
#[derive(Debug, Clone)]
pub struct CombatLki {
    pub is_attacker: bool,
    pub defender: Option<DefenderId>,
    pub blocked_attackers: Vec<CardId>,
}

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
    /// (attacker CardId, defender — player or permanent)
    pub attackers: Vec<(CardId, DefenderId)>,
    /// (blocker CardId, attacker CardId)
    pub blockers: Vec<(CardId, CardId)>,
    /// Damage assignment order: attacker → ordered list of blockers.
    /// The attacker must assign lethal to each blocker in order before
    /// moving to the next. Set after blocker declaration.
    #[serde(default)]
    pub damage_order: HashMap<CardId, Vec<CardId>>,
    /// Last-known-information cache: snapshots of creatures that left combat.
    /// Persists until combat ends (cleared in `clear()`).
    #[serde(skip)]
    pub lki_cache: HashMap<CardId, CombatLki>,
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
        self.damage_order.clear();
        self.lki_cache.clear();
    }

    /// Clear combat state, including the `attacking_player` flag on each attacker card.
    pub fn clear_with_cards(&mut self, cards: &mut [crate::card::CardInstance]) {
        for &(attacker_id, _) in &self.attackers {
            cards[attacker_id.index()].attacking_player = None;
        }
        // Preserve lki_cache across clear_with_cards (persists until end of combat)
        let lki = std::mem::take(&mut self.lki_cache);
        self.clear();
        self.lki_cache = lki;
    }

    pub fn declare_attacker(&mut self, attacker: CardId, defending: DefenderId) {
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

    pub fn get_attackers_for(&self, blocker: CardId) -> Vec<CardId> {
        self.blockers
            .iter()
            .filter(|(b, _)| *b == blocker)
            .map(|(_, a)| *a)
            .collect()
    }

    pub fn has_attackers(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// Snapshot a creature's combat role before it leaves the battlefield.
    pub fn save_lki(&mut self, card_id: CardId) -> Option<CombatLki> {
        // Check if attacker
        if let Some((_, defender)) = self.attackers.iter().find(|(a, _)| *a == card_id) {
            let lki = CombatLki {
                is_attacker: true,
                defender: Some(*defender),
                blocked_attackers: vec![],
            };
            self.lki_cache.insert(card_id, lki.clone());
            return Some(lki);
        }
        // Check if blocker
        let blocked: Vec<CardId> = self
            .blockers
            .iter()
            .filter(|(b, _)| *b == card_id)
            .map(|(_, a)| *a)
            .collect();
        if !blocked.is_empty() {
            let lki = CombatLki {
                is_attacker: false,
                defender: None,
                blocked_attackers: blocked,
            };
            self.lki_cache.insert(card_id, lki.clone());
            return Some(lki);
        }
        None
    }

    /// Get LKI for a creature that left combat.
    pub fn get_combat_lki(&self, card_id: CardId) -> Option<&CombatLki> {
        self.lki_cache.get(&card_id)
    }

    /// Check if a creature was (or is) attacking in this combat.
    pub fn was_attacking(&self, card_id: CardId) -> bool {
        self.attackers.iter().any(|(a, _)| *a == card_id)
            || self
                .lki_cache
                .get(&card_id)
                .map_or(false, |l| l.is_attacker)
    }

    /// Check if a creature was (or is) blocking in this combat.
    pub fn was_blocking(&self, card_id: CardId) -> bool {
        self.blockers.iter().any(|(b, _)| *b == card_id)
            || self
                .lki_cache
                .get(&card_id)
                .map_or(false, |l| !l.is_attacker)
    }

    /// Remove attackers/blockers that are no longer on the battlefield or are
    /// no longer creatures. Also cleans up damage_order keys. Returns `true`
    /// if any combatant was removed.
    ///
    /// Mirrors Java Forge's `Combat.removeAbsentCombatants()`.
    pub fn remove_absent_combatants(&mut self, cards: &[crate::card::CardInstance]) -> bool {
        let before_attackers = self.attackers.len();
        let before_blockers = self.blockers.len();

        self.attackers.retain(|&(id, _)| {
            let card = &cards[id.index()];
            card.zone == ZoneType::Battlefield && card.is_creature()
        });
        self.blockers.retain(|&(id, _)| {
            let card = &cards[id.index()];
            card.zone == ZoneType::Battlefield && card.is_creature()
        });

        // Clean damage_order keys for removed attackers
        let attacker_ids: HashSet<CardId> = self.attackers.iter().map(|(a, _)| *a).collect();
        self.damage_order.retain(|k, _| attacker_ids.contains(k));

        // Also remove dead blockers from damage_order values
        let blocker_ids: HashSet<CardId> = self.blockers.iter().map(|(b, _)| *b).collect();
        for order in self.damage_order.values_mut() {
            order.retain(|b| blocker_ids.contains(b));
        }

        self.attackers.len() != before_attackers || self.blockers.len() != before_blockers
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
        agents: &mut [Box<dyn PlayerAgent>],
        first_strike_only: bool,
        as_unblocked_choices: &HashSet<CardId>,
    ) -> Vec<CombatDamageEvent> {
        // Fog effect: skip all combat damage this turn (issue #22).
        if game.prevent_all_combat_damage {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut blocker_damage_allocations: HashMap<(CardId, CardId), i32> = HashMap::new();
        let mut computed_blocker_allocations: HashSet<CardId> = HashSet::new();
        // Java parity: combat damage in a step is simultaneous, so replacement checks
        // like Phyrexian Unlife's life condition must use life totals from step start.
        let life_at_step_start: Vec<i32> = game.players.iter().map(|p| p.life).collect();

        for (attacker_id, defender) in self.attackers.clone() {
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
            let attacker_has_trample = attacker.has_trample();
            let attacker_has_deathtouch = attacker.has_deathtouch();
            let attacker_has_lifelink = attacker.has_lifelink();
            let defending_player = defender.controlling_player(game);
            let attacker_has_infect_for_player = attacker.has_infect()
                || crate::staticability::static_ability_infect_damage::is_infect_damage_with_life_override(
                    game,
                    &game.cards,
                    defending_player,
                    attacker.controller,
                    life_at_step_start.get(defending_player.index()).copied(),
                );
            let attacker_has_infect_for_creature = attacker.has_infect();
            let attacker_has_wither = attacker.has_wither()
                || crate::staticability::static_ability_wither_damage::is_wither_damage(
                    &game.cards,
                    attacker,
                );
            let attacker_toxic_count = attacker.get_toxic_count();
            let attacker_controller = attacker.controller;
            let can_divide_damage_as_choose = attacker.has_keyword(
                "You may assign CARDNAME's combat damage divided as you choose among defending player and/or any number of creatures they control.",
            );
            let can_assign_unblocked_to_creature = attacker.has_keyword(
                "If CARDNAME is unblocked, you may have it assign its combat damage to a creature defending player controls.",
            );
            let has_trample_planeswalker = attacker.has_keyword("Trample:Planeswalker");

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
            } else if let Some(ordered) = self.damage_order.get(&attacker_id) {
                // Use player-chosen damage assignment order
                ordered.clone()
            } else {
                self.get_blockers_for(attacker_id)
            };

            if blockers.is_empty() {
                // Unblocked — damage goes to defender (player or permanent)
                if !attacker_deals_damage || attacker_power <= 0 {
                    continue;
                }

                let defending_creatures = defending_player_creatures(game, defender);
                if can_divide_damage_as_choose
                    && !defending_creatures.is_empty()
                    && agents[attacker_controller.index()].confirm_action(
                        attacker_controller,
                        Some("AlternativeDamageAssignment"),
                        &format!(
                            "Assign {} combat damage divided as you choose among defending player and/or creatures they control?",
                            game.card(attacker_id).card_name
                        ),
                        &[],
                        Some(&game.card(attacker_id).card_name),
                        None,
                    )
                {
                    let assignments = agents[attacker_controller.index()].assign_combat_damage(
                        game,
                        attacker_controller,
                        attacker_id,
                        &defending_creatures,
                        Some(DefenderId::Player(defending_player)),
                        attacker_power,
                    );
                    let (to_creatures, to_player) = validate_damage_assignment(
                        game,
                        attacker_id,
                        &defending_creatures,
                        Some(DefenderId::Player(defending_player)),
                        attacker_power,
                        &assignments,
                    );

                    for &(target_id, dmg) in &to_creatures {
                        deal_combat_damage_to_card(
                            game,
                            attacker_id,
                            target_id,
                            dmg,
                            attacker_has_deathtouch,
                            attacker_has_lifelink,
                            attacker_controller,
                            attacker_has_wither || attacker_has_infect_for_creature,
                        );
                        events.push(CombatDamageEvent {
                            source: attacker_id,
                            target_player: None,
                            target_card: Some(target_id),
                            amount: dmg,
                            is_combat: true,
                            lifelink_player: if attacker_has_lifelink {
                                Some(attacker_controller)
                            } else {
                                None
                            },
                            lifelink_amount: if attacker_has_lifelink { dmg } else { 0 },
                        });
                    }
                    if to_player > 0 {
                        deal_combat_damage_to_player(
                            game,
                            defending_player,
                            to_player,
                            attacker_has_lifelink,
                            attacker_controller,
                            attacker_has_infect_for_player,
                            attacker_toxic_count,
                        );
                        events.push(CombatDamageEvent {
                            source: attacker_id,
                            target_player: Some(defending_player),
                            target_card: None,
                            amount: to_player,
                            is_combat: true,
                            lifelink_player: if attacker_has_lifelink {
                                Some(attacker_controller)
                            } else {
                                None
                            },
                            lifelink_amount: if attacker_has_lifelink { to_player } else { 0 },
                        });
                        if game.card(attacker_id).is_commander {
                            *game
                                .player_mut(defending_player)
                                .commander_damage_received
                                .entry(attacker_id.0)
                                .or_insert(0) += to_player;
                        }
                    }
                    continue;
                }

                if can_assign_unblocked_to_creature
                    && !self.is_blocked(attacker_id)
                    && !defending_creatures.is_empty()
                    && agents[attacker_controller.index()].confirm_action(
                        attacker_controller,
                        Some("AlternativeDamageAssignment"),
                        &format!(
                            "Assign {} combat damage to a creature defending player controls?",
                            game.card(attacker_id).card_name
                        ),
                        &[],
                        Some(&game.card(attacker_id).card_name),
                        None,
                    )
                {
                    if let Some(chosen) =
                        agents[attacker_controller.index()].choose_target_card(
                            attacker_controller,
                            &defending_creatures,
                        )
                    {
                        deal_combat_damage_to_card(
                            game,
                            attacker_id,
                            chosen,
                            attacker_power,
                            attacker_has_deathtouch,
                            attacker_has_lifelink,
                            attacker_controller,
                            attacker_has_wither || attacker_has_infect_for_creature,
                        );
                        events.push(CombatDamageEvent {
                            source: attacker_id,
                            target_player: None,
                            target_card: Some(chosen),
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
                        continue;
                    }
                }
                match defender {
                    DefenderId::Player(defending_player) => {
                        deal_combat_damage_to_player(
                            game,
                            defending_player,
                            attacker_power,
                            attacker_has_lifelink,
                            attacker_controller,
                            attacker_has_infect_for_player,
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
                    }
                    DefenderId::Permanent(target_id) => {
                        // Damage to planeswalker/battle
                        deal_combat_damage_to_card(
                            game,
                            attacker_id,
                            target_id,
                            attacker_power,
                            attacker_has_deathtouch,
                            attacker_has_lifelink,
                            attacker_controller,
                            attacker_has_wither || attacker_has_infect_for_creature,
                        );
                        events.push(CombatDamageEvent {
                            source: attacker_id,
                            target_player: None,
                            target_card: Some(target_id),
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
                    }
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
                // Java-parity full damage assignment callback:
                // - prompt for exact assignment when needed (trample or multi-block)
                // - validate strictly (panic on invalid response; no fallback)
                let mut alive_blockers: Vec<CardId> = blockers
                    .iter()
                    .copied()
                    .filter(|&bid| game.card(bid).zone == ZoneType::Battlefield)
                    .collect();
                let mut effective_defender = defender;
                if has_trample_planeswalker {
                    if let DefenderId::Permanent(target_id) = defender {
                        if !alive_blockers.contains(&target_id) {
                            alive_blockers.push(target_id);
                        }
                        effective_defender = DefenderId::Player(defending_player);
                    }
                }

                let defending_creatures = defending_player_creatures(game, effective_defender);
                let use_divide_as_choose = can_divide_damage_as_choose
                    && !defending_creatures.is_empty()
                    && agents[attacker_controller.index()].confirm_action(
                        attacker_controller,
                        Some("AlternativeDamageAssignment"),
                        &format!(
                            "Assign {} combat damage divided as you choose among defending player and/or creatures they control?",
                            game.card(attacker_id).card_name
                        ),
                        &[],
                        Some(&game.card(attacker_id).card_name),
                        None,
                    );
                if use_divide_as_choose {
                    for cid in defending_creatures {
                        if !alive_blockers.contains(&cid) {
                            alive_blockers.push(cid);
                        }
                    }
                }

                let can_assign_to_defender = attacker_has_trample || use_divide_as_choose;
                if alive_blockers.is_empty() && !can_assign_to_defender {
                    continue;
                }
                let must_prompt_assignment = can_assign_to_defender || alive_blockers.len() > 1;

                let assignments = if must_prompt_assignment {
                    let controller = game.card(attacker_id).controller;
                    let defender_for_prompt = if can_assign_to_defender {
                        Some(effective_defender)
                    } else {
                        None
                    };
                    agents[controller.index()].assign_combat_damage(
                        game,
                        controller,
                        attacker_id,
                        &alive_blockers,
                        defender_for_prompt,
                        remaining_damage,
                    )
                } else if let Some(&only_blocker) = alive_blockers.first() {
                    vec![(Some(only_blocker), remaining_damage)]
                } else if can_assign_to_defender {
                    vec![(None, remaining_damage)]
                } else {
                    Vec::new()
                };

                let (damage_assignments, defender_damage) = validate_damage_assignment(
                    game,
                    attacker_id,
                    &alive_blockers,
                    can_assign_to_defender.then_some(effective_defender),
                    remaining_damage,
                    &assignments,
                );

                // --- Pre-compute blocker → attacker damage BEFORE applying any damage ---
                // Combat damage is simultaneous (rule 510.2). We must read blocker
                // powers now, before wither/infect -1/-1 counters from attacker
                // damage modify them.
                struct BlockerDamageInfo {
                    blocker_id: CardId,
                    power: i32,
                    has_deathtouch: bool,
                    has_lifelink: bool,
                    has_wither_or_infect: bool,
                    controller: PlayerId,
                }
                let mut blocker_damage_infos: Vec<BlockerDamageInfo> = Vec::new();
                for &blocker_id in &blockers {
                    if game.card(blocker_id).zone != ZoneType::Battlefield {
                        continue;
                    }
                    let blocker_card = game.card(blocker_id);
                    if crate::staticability::static_ability_assign_no_combat_damage::assign_no_combat_damage(
                        &game.cards,
                        blocker_card,
                    ) {
                        continue;
                    }
                    let blocker_has_fs = blocker_card.has_first_strike();
                    let blocker_has_ds = blocker_card.has_double_strike();
                    let blocker_deals = if first_strike_only {
                        blocker_has_fs || blocker_has_ds
                    } else {
                        !blocker_has_fs || blocker_has_ds
                    };
                    if !blocker_deals {
                        continue;
                    }
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
                        if !computed_blocker_allocations.contains(&blocker_id) {
                            let per_attacker = compute_blocker_damage_allocations(
                                self,
                                game,
                                agents,
                                first_strike_only,
                                blocker_id,
                                blocker_power,
                            );
                            for (target_attacker, dmg) in per_attacker {
                                blocker_damage_allocations
                                    .insert((blocker_id, target_attacker), dmg);
                            }
                            computed_blocker_allocations.insert(blocker_id);
                        }
                        let assigned_to_this_attacker = blocker_damage_allocations
                            .get(&(blocker_id, attacker_id))
                            .copied()
                            .unwrap_or(0);
                        if assigned_to_this_attacker <= 0 {
                            continue;
                        }
                        let blocker_has_infect = blocker_card.has_infect();
                        let blocker_has_wither = blocker_card.has_wither()
                            || crate::staticability::static_ability_wither_damage::is_wither_damage(
                                &game.cards,
                                blocker_card,
                            );
                        blocker_damage_infos.push(BlockerDamageInfo {
                            blocker_id,
                            power: assigned_to_this_attacker,
                            has_deathtouch: blocker_card.has_deathtouch(),
                            has_lifelink: blocker_card.has_lifelink(),
                            has_wither_or_infect: blocker_has_wither || blocker_has_infect,
                            controller: blocker_card.controller,
                        });
                    }
                }

                // Now apply all damage (attacker → blockers, then blockers → attacker)
                // using pre-computed power values.
                for &(blocker_id, damage_to_blocker) in &damage_assignments {
                    deal_combat_damage_to_card(
                        game,
                        attacker_id,
                        blocker_id,
                        damage_to_blocker,
                        attacker_has_deathtouch,
                        attacker_has_lifelink,
                        attacker_controller,
                        attacker_has_wither || attacker_has_infect_for_creature,
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

                if defender_damage > 0 {
                    match effective_defender {
                        DefenderId::Player(defending_player) => {
                            deal_combat_damage_to_player(
                                game,
                                defending_player,
                                defender_damage,
                                attacker_has_lifelink,
                                attacker_controller,
                                attacker_has_infect_for_player,
                                attacker_toxic_count,
                            );
                            events.push(CombatDamageEvent {
                                source: attacker_id,
                                target_player: Some(defending_player),
                                target_card: None,
                                amount: defender_damage,
                                is_combat: true,
                                lifelink_player: if attacker_has_lifelink {
                                    Some(attacker_controller)
                                } else {
                                    None
                                },
                                lifelink_amount: if attacker_has_lifelink {
                                    defender_damage
                                } else {
                                    0
                                },
                            });
                            if game.card(attacker_id).is_commander {
                                *game
                                    .player_mut(defending_player)
                                    .commander_damage_received
                                    .entry(attacker_id.0)
                                    .or_insert(0) += defender_damage;
                            }
                        }
                        DefenderId::Permanent(target_id) => {
                            deal_combat_damage_to_card(
                                game,
                                attacker_id,
                                target_id,
                                defender_damage,
                                attacker_has_deathtouch,
                                attacker_has_lifelink,
                                attacker_controller,
                                attacker_has_wither || attacker_has_infect_for_creature,
                            );
                            events.push(CombatDamageEvent {
                                source: attacker_id,
                                target_player: None,
                                target_card: Some(target_id),
                                amount: defender_damage,
                                is_combat: true,
                                lifelink_player: if attacker_has_lifelink {
                                    Some(attacker_controller)
                                } else {
                                    None
                                },
                                lifelink_amount: if attacker_has_lifelink {
                                    defender_damage
                                } else {
                                    0
                                },
                            });
                        }
                    }
                }

                for info in &blocker_damage_infos {
                    // Blocker may have been removed by an SBA or replacement
                    if game.card(info.blocker_id).zone != ZoneType::Battlefield {
                        continue;
                    }
                    deal_combat_damage_to_card(
                        game,
                        info.blocker_id,
                        attacker_id,
                        info.power,
                        info.has_deathtouch,
                        info.has_lifelink,
                        info.controller,
                        info.has_wither_or_infect,
                    );
                    events.push(CombatDamageEvent {
                        source: info.blocker_id,
                        target_player: None,
                        target_card: Some(attacker_id),
                        amount: info.power,
                        is_combat: true,
                        lifelink_player: if info.has_lifelink {
                            Some(info.controller)
                        } else {
                            None
                        },
                        lifelink_amount: if info.has_lifelink { info.power } else { 0 },
                    });
                }

                // Note: non-trample excess is validated/flushed to last blocker;
                // trample excess is applied to defender.
            }
        }

        events
    }
}

fn validate_damage_assignment(
    game: &GameState,
    attacker_id: CardId,
    blockers_in_order: &[CardId],
    defender: Option<DefenderId>,
    total_damage: i32,
    assignments: &[(Option<CardId>, i32)],
) -> (Vec<(CardId, i32)>, i32) {
    if total_damage <= 0 {
        return (Vec::new(), 0);
    }

    let mut per_blocker: HashMap<CardId, i32> = HashMap::new();
    let mut defender_damage = 0;
    let mut assigned_total = 0;

    let mut invalid = false;

    for &(assignee, amount) in assignments {
        if amount < 0 {
            invalid = true;
            break;
        }
        if amount == 0 {
            continue;
        }
        assigned_total += amount;
        match assignee {
            Some(blocker_id) => {
                if !blockers_in_order.contains(&blocker_id) {
                    invalid = true;
                    break;
                }
                *per_blocker.entry(blocker_id).or_insert(0) += amount;
            }
            None => {
                if defender.is_none() {
                    invalid = true;
                    break;
                }
                defender_damage += amount;
            }
        }
    }

    if assigned_total != total_damage {
        invalid = true;
    }

    let has_deathtouch = game.card(attacker_id).has_deathtouch();
    let mut can_move_to_next = true;
    for &blocker_id in blockers_in_order {
        if game.card(blocker_id).zone != ZoneType::Battlefield {
            continue;
        }
        if crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
            &game.cards,
            game.card(blocker_id),
            game.card(attacker_id),
        ) {
            continue;
        }

        let assigned = per_blocker.get(&blocker_id).copied().unwrap_or(0);
        let lethal = if has_deathtouch {
            1
        } else if game.card(blocker_id).type_line.is_planeswalker() {
            game
                .card(blocker_id)
                .counter_count(&crate::card::CounterType::Loyalty)
                .max(0)
        } else {
            let blocker = game.card(blocker_id);
            (blocker.toughness() - blocker.damage).max(0)
        };

        if !can_move_to_next && assigned > 0 {
            invalid = true;
            break;
        }
        if assigned < lethal {
            can_move_to_next = false;
        }
    }

    if defender_damage > 0 && !can_move_to_next {
        invalid = true;
    }

    if invalid {
        return fallback_damage_assignment(game, attacker_id, blockers_in_order, defender, total_damage);
    }

    let mut ordered_blocker_assignments: Vec<(CardId, i32)> = Vec::new();
    for &blocker_id in blockers_in_order {
        if let Some(amount) = per_blocker.get(&blocker_id).copied() {
            if amount > 0 {
                ordered_blocker_assignments.push((blocker_id, amount));
            }
        }
    }

    (ordered_blocker_assignments, defender_damage)
}

fn fallback_damage_assignment(
    game: &GameState,
    attacker_id: CardId,
    blockers_in_order: &[CardId],
    defender: Option<DefenderId>,
    total_damage: i32,
) -> (Vec<(CardId, i32)>, i32) {
    if total_damage <= 0 {
        return (Vec::new(), 0);
    }

    let mut assignments: Vec<(CardId, i32)> = Vec::new();
    let mut damage_left = total_damage;
    let has_deathtouch = game.card(attacker_id).has_deathtouch();

    for &blocker_id in blockers_in_order {
        if damage_left <= 0 {
            break;
        }
        if game.card(blocker_id).zone != ZoneType::Battlefield {
            continue;
        }
        if crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
            &game.cards,
            game.card(blocker_id),
            game.card(attacker_id),
        ) {
            continue;
        }

        let lethal = if has_deathtouch {
            1
        } else if game.card(blocker_id).type_line.is_planeswalker() {
            game
                .card(blocker_id)
                .counter_count(&crate::card::CounterType::Loyalty)
                .max(0)
        } else {
            let blocker = game.card(blocker_id);
            (blocker.toughness() - blocker.damage).max(0)
        };
        let assign = lethal.min(damage_left);
        if assign > 0 {
            assignments.push((blocker_id, assign));
            damage_left -= assign;
        }
    }

    if damage_left > 0 {
        if defender.is_some() {
            return (assignments, damage_left);
        }
        if let Some((_, amount)) = assignments.last_mut() {
            *amount += damage_left;
        } else if let Some(&first) = blockers_in_order.first() {
            assignments.push((first, damage_left));
        }
        return (assignments, 0);
    }

    (assignments, 0)
}

fn defending_player_creatures(game: &GameState, defender: DefenderId) -> Vec<CardId> {
    let defending_player = defender.controlling_player(game);
    game.cards_in_zone(ZoneType::Battlefield, defending_player)
        .iter()
        .copied()
        .filter(|&cid| game.card(cid).is_creature())
        .collect()
}

fn compute_blocker_damage_allocations(
    combat: &CombatState,
    game: &GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    first_strike_only: bool,
    blocker_id: CardId,
    blocker_power: i32,
) -> Vec<(CardId, i32)> {
    if blocker_power <= 0 {
        return Vec::new();
    }

    let blocker = game.card(blocker_id);
    let has_fs = blocker.has_first_strike();
    let has_ds = blocker.has_double_strike();
    let deals_this_step = if first_strike_only {
        has_fs || has_ds
    } else {
        !has_fs || has_ds
    };
    if !deals_this_step {
        return Vec::new();
    }

    let attackers_for_blocker: Vec<CardId> = combat
        .get_attackers_for(blocker_id)
        .into_iter()
        .filter(|&aid| game.card(aid).zone == ZoneType::Battlefield)
        .collect();
    if attackers_for_blocker.is_empty() {
        return Vec::new();
    }

    if attackers_for_blocker.len() == 1 {
        return vec![(attackers_for_blocker[0], blocker_power)];
    }

    let controller = blocker.controller;
    let assignments = agents[controller.index()].assign_combat_damage(
        game,
        controller,
        blocker_id,
        &attackers_for_blocker,
        None,
        blocker_power,
    );
    let (per_attacker, _to_defender) = validate_damage_assignment(
        game,
        blocker_id,
        &attackers_for_blocker,
        None,
        blocker_power,
        &assignments,
    );
    per_attacker
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
                    &game.cards,
                    card,
                    defending,
                )
        })
        .collect()
}

/// Get all possible defenders for the attacking player: opponent players
/// and planeswalkers controlled by opponents.
pub fn get_possible_defenders(game: &GameState, attacking_player: PlayerId) -> Vec<DefenderId> {
    let mut defenders = Vec::new();
    for pid in game.alive_players() {
        if pid == attacking_player {
            continue;
        }
        defenders.push(DefenderId::Player(pid));
        // Planeswalkers controlled by this opponent
        for &cid in game.cards_in_zone(ZoneType::Battlefield, pid) {
            let card = game.card(cid);
            if card.type_line.is_planeswalker() {
                defenders.push(DefenderId::Permanent(cid));
            }
        }
    }
    defenders
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
    // CantBlockBy static abilities (e.g. "CARDNAME can block only creatures with flying")
    if cant_block_by(game, attacker_id, blocker_id) {
        return false;
    }
    true
}

/// Check if any `CantBlockBy` static ability prevents `blocker_id` from blocking `attacker_id`.
/// Mirrors Java `StaticAbilityCantAttackBlock.cantBlockBy()`.
fn cant_block_by(game: &GameState, attacker_id: CardId, blocker_id: CardId) -> bool {
    let attacker = game.card(attacker_id);
    let blocker = game.card(blocker_id);

    for source in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.zone == ZoneType::Command)
    {
        for sa in &source.static_abilities {
            if sa.mode != StaticMode::CantBlockBy {
                continue;
            }

            // ValidAttacker$ — the attacker must match for this restriction to apply.
            if let Some(valid_attacker) = sa.params.get("ValidAttacker") {
                if !matches_valid_card(attacker, valid_attacker, source) {
                    continue;
                }
            }

            // ValidBlocker$ — if the blocker matches ANY comma-separated validator,
            // the restriction applies to this blocker.
            if let Some(valid_blocker) = sa.params.get("ValidBlocker") {
                let blocker_matches = valid_blocker
                    .split(',')
                    .any(|v| matches_valid_card(blocker, v.trim(), source));
                if !blocker_matches {
                    // Blocker doesn't match any validator — restriction doesn't target it.
                    continue;
                }
            }

            // Both conditions met: this block is illegal.
            return true;
        }
    }
    false
}

/// Check if a card matches a `ValidAttacker$` / `ValidBlocker$` filter string.
/// Handles `Card.Self` (card is the source) and delegates to `card_has_property`.
fn matches_valid_card(
    card: &crate::card::CardInstance,
    filter: &str,
    source: &crate::card::CardInstance,
) -> bool {
    valid_filter::matches_valid_card(filter, card, source)
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
        if lifelink
            && !crate::staticability::static_ability_cant_gain_lose_pay_life::cant_gain_life(
                game,
                source_controller,
            )
        {
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
        if lifelink
            && !crate::staticability::static_ability_cant_gain_lose_pay_life::cant_gain_life(
                game,
                source_controller,
            )
        {
            game.player_mut(source_controller).gain_life(amount);
        }
        // Record damage in source's damage history
        game.card_mut(source)
            .damage_history
            .record_damage(amount, true);
    }
}

// ── Lure / Must-Block helpers ─────────────────────────────────────────

/// What kind of lure effect an attacker has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LureType {
    /// No lure effect.
    None,
    /// "CARDNAME must be blocked if able." — at least 1 blocker required.
    MustBeBlockedIfAble,
    /// "All creatures able to block CARDNAME do so." — ALL legal blockers must block it.
    AllMustBlock,
}

/// Determine the lure type of an attacker based on its keywords.
pub fn get_lure_type(card: &crate::card::CardInstance) -> LureType {
    for kw in card
        .keywords
        .iter()
        .chain(card.granted_keywords.iter())
        .chain(card.pump_keywords.iter())
    {
        let lower = kw.to_lowercase();
        if lower.contains("all creatures able to block") && lower.contains("do so") {
            return LureType::AllMustBlock;
        }
        if lower.contains("must be blocked if able") {
            return LureType::MustBeBlockedIfAble;
        }
    }
    LureType::None
}

/// Get attackers that `blocker_id` MUST block (if able).
///
/// Checks:
/// 1. Attackers with Lure-type keywords where blocker can legally block
/// 2. Explicit `must_block_cards` on the blocker
/// 3. The `must_block` flag (generic must-block from statics/effects)
///
/// Returns the list of attacker CardIds that the blocker is required to block.
pub fn compute_must_block_targets(
    game: &GameState,
    combat: &CombatState,
    blocker_id: CardId,
) -> Vec<CardId> {
    let mut targets = Vec::new();
    let blocker = game.card(blocker_id);

    // Check all current attackers for lure keywords
    for &(attacker_id, _) in &combat.attackers {
        let attacker = game.card(attacker_id);
        let lure = get_lure_type(attacker);
        match lure {
            LureType::AllMustBlock => {
                if can_creature_block(game, blocker_id, attacker_id) {
                    targets.push(attacker_id);
                }
            }
            LureType::MustBeBlockedIfAble => {
                if can_creature_block(game, blocker_id, attacker_id) {
                    targets.push(attacker_id);
                }
            }
            LureType::None => {}
        }
    }

    // Check explicit must_block_cards on the blocker
    for &attacker_id in &blocker.must_block_cards {
        if combat.is_attacking(attacker_id)
            && can_creature_block(game, blocker_id, attacker_id)
            && !targets.contains(&attacker_id)
        {
            targets.push(attacker_id);
        }
    }

    targets
}

/// Validate blocker assignments and return invalid (blocker, attacker) pairs to remove.
///
/// Checks:
/// 1. Menace: attacker with Menace must be blocked by 2+ creatures or 0
/// 2. Can't block alone: blocker has keyword and is only blocker on its attacker
///
/// Called after blocker declaration to clean up illegal blocks.
pub fn validate_blocks(game: &GameState, combat: &CombatState) -> Vec<(CardId, CardId)> {
    let mut invalid = Vec::new();

    for &(attacker_id, _) in &combat.attackers {
        let blockers_for = combat.get_blockers_for(attacker_id);
        let num_blockers = blockers_for.len();

        if num_blockers == 0 {
            continue;
        }

        // Menace: must be blocked by 2+ creatures
        if game.card(attacker_id).has_menace() && num_blockers == 1 {
            invalid.push((blockers_for[0], attacker_id));
            continue;
        }

        // Check blockers with "can't block alone" keyword
        for &blocker_id in &blockers_for {
            let blocker = game.card(blocker_id);
            let cant_block_alone = blocker
                .keywords
                .iter()
                .chain(blocker.granted_keywords.iter())
                .chain(blocker.pump_keywords.iter())
                .any(|kw| kw.to_lowercase().contains("can't block alone"));

            if cant_block_alone && num_blockers == 1 {
                invalid.push((blocker_id, attacker_id));
            }
        }
    }

    invalid
}
