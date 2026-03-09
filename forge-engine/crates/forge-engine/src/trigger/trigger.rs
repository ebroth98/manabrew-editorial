use std::collections::BTreeMap;

use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::event::RunParams;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Mirrors Java's abstract Trigger class.
/// In Java, each TriggerType has a subclass (TriggerChangesZone, TriggerPhase, etc.)
/// with a performTest() override. In Rust, TriggerMode enum dispatch replaces this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: u32,
    pub mode: TriggerMode,
    /// Raw parsed parameters — mirrors Java's mapParams: Map<String,String>.
    pub params: BTreeMap<String, String>,
    /// Zones where host card must be for trigger to be active.
    /// Default: [Battlefield].
    pub active_zones: Vec<ZoneType>,
    /// SVar name to execute — mirrors Java's Execute$ → overridingAbility.
    pub execute: String,
    /// Whether trigger is optional (has OptionalDecider$).
    pub optional: bool,
    /// Trigger description text.
    pub description: String,
    /// Whether this trigger is intrinsic to the card.
    pub intrinsic: bool,
}

/// Replaces Java's Trigger subclass hierarchy.
/// Each variant holds the parsed parameters specific to that trigger type,
/// and perform_test() dispatches to variant-specific matching logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerMode {
    ChangesZone {
        origin: Option<ZoneType>,
        destination: Option<ZoneType>,
        valid_card: Option<String>,
    },
    Phase {
        phase: Option<PhaseType>,
        valid_player: Option<String>,
    },
    SpellCast {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    Attacks {
        valid_card: Option<String>,
    },
    DamageDone {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    /// A spell was countered (SP$ Counter).
    Countered {
        valid_card: Option<String>,
        valid_cause: Option<String>,
        valid_sa: Option<String>,
    },
    // ── New trigger modes (issue #19) ──
    /// A creature blocks an attacker.
    Blocks {
        valid_card: Option<String>,
        valid_blocked: Option<String>,
    },
    /// An attacker is blocked.
    AttackerBlocked {
        valid_card: Option<String>,
    },
    /// An attacker is not blocked.
    AttackerUnblocked {
        valid_card: Option<String>,
    },
    /// A player gained life.
    LifeGained {
        valid_player: Option<String>,
    },
    /// A player lost life.
    LifeLost {
        valid_player: Option<String>,
    },
    /// A counter was added to a permanent.
    CounterAdded {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A counter was removed from a permanent.
    CounterRemoved {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A permanent was sacrificed.
    Sacrificed {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A card was drawn.
    Drawn {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A card was milled.
    Milled {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A permanent was tapped.
    Taps {
        valid_card: Option<String>,
    },
    /// A permanent was untapped.
    Untaps {
        valid_card: Option<String>,
    },
    /// A DFC was transformed.
    Transformed {
        valid_card: Option<String>,
    },
    /// An aura/equipment was attached.
    Attached {
        valid_card: Option<String>,
    },
    /// An aura/equipment was unattached.
    Unattached {
        valid_card: Option<String>,
    },
    /// A land was played.
    LandPlayed {
        valid_card: Option<String>,
    },
    /// A permanent became the target of a spell or ability.
    BecomesTarget {
        valid_card: Option<String>,
    },
    /// A permanent was tapped for mana.
    TapsForMana {
        valid_card: Option<String>,
    },
    /// An activated ability was activated.
    AbilityActivated {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    /// A creature explored.
    Explored {
        valid_card: Option<String>,
    },
    /// A creature became monstrous.
    BecomeMonstrous {
        valid_card: Option<String>,
    },
    /// A player became the monarch.
    BecomeMonarch {
        valid_player: Option<String>,
    },
    /// Damage was dealt to a player/creature for the first time this turn.
    DamageDealtOnce {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    /// A permanent was destroyed.
    Destroyed {
        valid_card: Option<String>,
    },
    /// A card was exiled.
    Exiled {
        valid_card: Option<String>,
    },
    /// A token was created.
    TokenCreated {
        valid_card: Option<String>,
    },
    /// A spell was copied (Storm, Replicate, etc.) — for Magecraft triggers.
    SpellCopied {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    // ── New trigger modes (issue #54) ──
    AttackersDeclared {
        valid_player: Option<String>,
        valid_attackers: Option<String>,
        valid_attackers_amount: Option<String>,
    },
    BlockersDeclared,
    ChangesZoneAll {
        origin: Option<ZoneType>,
        destination: Option<ZoneType>,
        valid_card: Option<String>,
    },
    ChangesController {
        valid_card: Option<String>,
    },
    TurnBegin {
        valid_player: Option<String>,
    },
    DamageDoneOnce {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    SpellCastAll {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    LifeLostAll {
        valid_player: Option<String>,
    },
    CounterAddedOnce {
        valid_card: Option<String>,
        counter_type: Option<String>,
        valid_source: Option<String>,
    },
    DiscardedAll {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    SacrificedOnce {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    Cycled {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    PhasedIn {
        valid_card: Option<String>,
    },
    PhasedOut {
        valid_card: Option<String>,
    },
    Always,
    Immediate,
    Surveil {
        valid_player: Option<String>,
    },
    Scry {
        valid_player: Option<String>,
    },
    Foretell {
        valid_card: Option<String>,
    },
    SearchedLibrary {
        valid_player: Option<String>,
    },
    Shuffled {
        valid_player: Option<String>,
    },
    ManaAdded {
        valid_card: Option<String>,
    },
    TokenCreatedOnce {
        valid_card: Option<String>,
    },
    TapAll {
        valid_card: Option<String>,
    },
    UntapAll {
        valid_card: Option<String>,
    },
    BecomesTargetOnce {
        valid_card: Option<String>,
    },
    AttackerBlockedByCreature {
        valid_card: Option<String>,
        valid_blocked: Option<String>,
    },
    AttackerBlockedOnce {
        valid_card: Option<String>,
    },
    AttackerUnblockedOnce {
        valid_card: Option<String>,
    },
    SpellCastOnce {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellCastOfType {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    DamageAll {
        valid_source: Option<String>,
        valid_target: Option<String>,
    },
    DamagePreventedOnce {
        valid_card: Option<String>,
    },
    ExcessDamage {
        valid_source: Option<String>,
        valid_target: Option<String>,
    },
    LifeGainedAll {
        valid_player: Option<String>,
    },
    CounterRemovedOnce {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A creature was exerted (Mirrors Java TriggerExerted).
    Exerted {
        valid_card: Option<String>,
    },
    /// A player collected evidence.
    CollectEvidence {
        valid_player: Option<String>,
    },
    /// A player foraged.
    Forage {
        valid_player: Option<String>,
    },
    /// A creature enlisted another creature.
    Enlisted {
        valid_card: Option<String>,
        valid_enlisted: Option<String>,
    },
    /// A player flipped a coin.
    FlippedCoin {
        valid_player: Option<String>,
        valid_result: Option<String>,
    },
    /// A player rolled a die.
    RolledDie {
        valid_player: Option<String>,
        valid_result: Option<String>,
        valid_sides: Option<String>,
    },
    /// A player completed a die-roll action.
    RolledDieOnce {
        valid_player: Option<String>,
        valid_result: Option<String>,
        valid_sides: Option<String>,
    },
    /// Mana was expended (Expend N mechanic). Fires when cumulative mana spent reaches Amount.
    ManaExpend {
        valid_player: Option<String>,
        amount: i32,
    },
}

/// Check an optional ValidCard$ filter against a card from RunParams.
/// Returns false if the filter exists but the card doesn't match.
fn check_card_filter(
    filter: &Option<String>,
    card_id: Option<CardId>,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) -> bool {
    if let Some(filter) = filter {
        if let Some(cid) = card_id {
            matches_valid_card(filter, cid, host_card, host_controller, game)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check an optional ValidPlayer$ filter against a player from RunParams.
/// Returns false if the filter exists but the player doesn't match.
fn check_player_filter(
    filter: &Option<String>,
    player_id: Option<PlayerId>,
    host_controller: PlayerId,
) -> bool {
    if let Some(filter) = filter {
        if let Some(pid) = player_id {
            matches_valid_player(filter, pid, host_controller)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check an optional CounterType$ filter against a counter type from RunParams.
fn check_counter_type_filter(expected: &Option<String>, actual: &Option<String>) -> bool {
    if let Some(expected) = expected {
        if let Some(actual) = actual {
            actual.eq_ignore_ascii_case(expected)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check a damage target filter that can match either a card or player target.
/// `strict_card_filter` controls whether card-specific filters (Card., Creature., etc.)
/// reject player targets.
fn check_damage_target(
    filter: &Option<String>,
    run_params: &RunParams,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
    strict_card_filter: bool,
) -> bool {
    let filter = match filter {
        Some(f) => f,
        None => return true,
    };
    if let Some(target_card) = run_params.damage_target_card {
        matches_valid_card(filter, target_card, host_card, host_controller, game)
    } else if strict_card_filter {
        let is_card_filter = filter.starts_with("Card.")
            || filter.starts_with("Creature.")
            || filter.starts_with("Permanent.")
            || filter.starts_with("Artifact.")
            || filter.starts_with("Enchantment.")
            || filter.starts_with("Planeswalker.");
        if is_card_filter {
            false
        } else if let Some(target_player) = run_params.damage_target_player {
            matches_valid_player(filter, target_player, host_controller)
        } else {
            false
        }
    } else if let Some(target_player) = run_params.damage_target_player {
        matches_valid_player(filter, target_player, host_controller)
    } else {
        false
    }
}

/// Check zone matches (origin and/or destination).
fn check_zone_filter(expected: &Option<ZoneType>, actual: Option<ZoneType>) -> bool {
    if let Some(expected) = expected {
        actual == Some(*expected)
    } else {
        true
    }
}

impl TriggerMode {
    /// Mirrors Java's Trigger.performTest() — each subclass overrides.
    /// In Rust, enum match replaces virtual dispatch.
    pub fn perform_test(
        &self,
        run_params: &RunParams,
        game: &GameState,
        host_card: CardId,
        host_controller: PlayerId,
    ) -> bool {
        match self {
            // ── Zone change triggers ──
            TriggerMode::ChangesZone {
                origin,
                destination,
                valid_card,
            }
            | TriggerMode::ChangesZoneAll {
                origin,
                destination,
                valid_card,
            } => {
                check_zone_filter(origin, run_params.origin)
                    && check_zone_filter(destination, run_params.destination)
                    && check_card_filter(
                        valid_card,
                        run_params.card,
                        host_card,
                        host_controller,
                        game,
                    )
            }

            // ── Phase trigger ──
            TriggerMode::Phase {
                phase,
                valid_player,
            } => {
                if let Some(expected_phase) = phase {
                    if run_params.phase != Some(*expected_phase) {
                        return false;
                    }
                }
                check_player_filter(valid_player, run_params.player, host_controller)
            }

            // ── Spell cast triggers (check spell_card + spell_controller) ──
            TriggerMode::SpellCast {
                valid_card,
                valid_activating_player,
            }
            | TriggerMode::SpellCastAll {
                valid_card,
                valid_activating_player,
            }
            | TriggerMode::SpellCastOnce {
                valid_card,
                valid_activating_player,
            }
            | TriggerMode::SpellCastOfType {
                valid_card,
                valid_activating_player,
            }
            | TriggerMode::SpellCopied {
                valid_card,
                valid_activating_player,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.spell_card,
                    host_card,
                    host_controller,
                    game,
                ) && check_player_filter(
                    valid_activating_player,
                    run_params.spell_controller,
                    host_controller,
                )
            }

            // ── Attacks trigger (check attacker) ──
            TriggerMode::Attacks { valid_card } => check_card_filter(
                valid_card,
                run_params.attacker,
                host_card,
                host_controller,
                game,
            ),

            // ── Attacker blocked/unblocked (check attacker) ──
            TriggerMode::AttackerBlocked { valid_card }
            | TriggerMode::AttackerUnblocked { valid_card } => check_card_filter(
                valid_card,
                run_params.attacker,
                host_card,
                host_controller,
                game,
            ),

            // ── Damage triggers with source + target + combat flag ──
            TriggerMode::DamageDone {
                valid_source,
                valid_target,
                combat_damage_only,
            }
            | TriggerMode::DamageDoneOnce {
                valid_source,
                valid_target,
                combat_damage_only,
            }
            | TriggerMode::DamageDealtOnce {
                valid_source,
                valid_target,
                combat_damage_only,
            } => {
                if *combat_damage_only && run_params.is_combat_damage != Some(true) {
                    return false;
                }
                check_card_filter(
                    valid_source,
                    run_params.damage_source,
                    host_card,
                    host_controller,
                    game,
                ) && check_damage_target(
                    valid_target,
                    run_params,
                    host_card,
                    host_controller,
                    game,
                    true,
                )
            }

            // ── Damage triggers without combat flag ──
            TriggerMode::DamageAll {
                valid_source,
                valid_target,
            }
            | TriggerMode::ExcessDamage {
                valid_source,
                valid_target,
            } => {
                check_card_filter(
                    valid_source,
                    run_params.damage_source,
                    host_card,
                    host_controller,
                    game,
                ) && check_damage_target(
                    valid_target,
                    run_params,
                    host_card,
                    host_controller,
                    game,
                    false,
                )
            }

            // ── Countered trigger ──
            TriggerMode::Countered {
                valid_card,
                valid_cause,
                valid_sa,
            } => {
                if !check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) {
                    return false;
                }
                if let Some(filter) = valid_cause {
                    if let Some(cause) = run_params.cause.as_ref() {
                        let Some(cause_card) = cause.source else {
                            return false;
                        };
                        if !matches_valid_card(filter, cause_card, host_card, host_controller, game)
                        {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(filter) = valid_sa {
                    if let Some(countered_sa) = run_params.spell_ability.as_ref() {
                        if !matches_valid_sa(filter, countered_sa) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }

            // ── Block triggers (check blocker + blocked_attacker) ──
            TriggerMode::Blocks {
                valid_card,
                valid_blocked,
            }
            | TriggerMode::AttackerBlockedByCreature {
                valid_card,
                valid_blocked,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.blocker,
                    host_card,
                    host_controller,
                    game,
                ) && check_card_filter(
                    valid_blocked,
                    run_params.blocked_attacker,
                    host_card,
                    host_controller,
                    game,
                )
            }

            // ── Card + player triggers (check run_params.card + run_params.player) ──
            TriggerMode::Sacrificed {
                valid_card,
                valid_player,
            }
            | TriggerMode::Drawn {
                valid_card,
                valid_player,
            }
            | TriggerMode::Milled {
                valid_card,
                valid_player,
            }
            | TriggerMode::DiscardedAll {
                valid_card,
                valid_player,
            }
            | TriggerMode::SacrificedOnce {
                valid_card,
                valid_player,
            }
            | TriggerMode::Cycled {
                valid_card,
                valid_player,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) && check_player_filter(valid_player, run_params.player, host_controller)
            }

            // ── Enlisted trigger (host card + enlisted card) ──
            TriggerMode::Enlisted {
                valid_card,
                valid_enlisted,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) && check_card_filter(
                    valid_enlisted,
                    run_params.enlisted,
                    host_card,
                    host_controller,
                    game,
                )
            }
            TriggerMode::FlippedCoin {
                valid_player,
                valid_result,
            } => {
                if !check_player_filter(valid_player, run_params.player, host_controller) {
                    return false;
                }
                if let Some(filter) = valid_result {
                    let Some(won) = run_params.coin_flip_won else {
                        return false;
                    };
                    let f = filter.trim();
                    if (f.eq_ignore_ascii_case("Win") || f.eq_ignore_ascii_case("Heads")) && !won {
                        return false;
                    }
                    if (f.eq_ignore_ascii_case("Lose") || f.eq_ignore_ascii_case("Tails")) && won {
                        return false;
                    }
                }
                true
            }
            TriggerMode::RolledDie {
                valid_player,
                valid_result,
                valid_sides,
            }
            | TriggerMode::RolledDieOnce {
                valid_player,
                valid_result,
                valid_sides,
            } => {
                if !check_player_filter(valid_player, run_params.player, host_controller) {
                    return false;
                }
                if let Some(filter) = valid_result {
                    let Some(result) = run_params.die_result else {
                        return false;
                    };
                    if !matches_amount(filter, result as usize) {
                        return false;
                    }
                }
                if let Some(filter) = valid_sides {
                    let Some(sides) = run_params.die_sides else {
                        return false;
                    };
                    if !matches_amount(filter, sides as usize) {
                        return false;
                    }
                }
                true
            }

            // ── Card + player triggers (check run_params.card + run_params.player for activated) ──
            TriggerMode::AbilityActivated {
                valid_card,
                valid_activating_player,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) && check_player_filter(
                    valid_activating_player,
                    run_params.player,
                    host_controller,
                )
            }

            // ── Counter triggers (card + counter_type) ──
            TriggerMode::CounterAdded {
                valid_card,
                counter_type,
            }
            | TriggerMode::CounterRemoved {
                valid_card,
                counter_type,
            }
            | TriggerMode::CounterRemovedOnce {
                valid_card,
                counter_type,
            } => {
                check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) && check_counter_type_filter(counter_type, &run_params.counter_type)
            }

            // ── CounterAddedOnce (card + counter_type + valid_source) ──
            TriggerMode::CounterAddedOnce {
                valid_card,
                counter_type,
                valid_source,
            } => {
                if !check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) {
                    return false;
                }
                if !check_counter_type_filter(counter_type, &run_params.counter_type) {
                    return false;
                }
                if let Some(filter) = valid_source {
                    if filter.eq_ignore_ascii_case("You") {
                        if let Some(cause) = run_params.cause_player {
                            if cause != host_controller {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
                true
            }

            // ── Card-only triggers (check run_params.card) ──
            TriggerMode::Taps { valid_card }
            | TriggerMode::Untaps { valid_card }
            | TriggerMode::Transformed { valid_card }
            | TriggerMode::Attached { valid_card }
            | TriggerMode::Unattached { valid_card }
            | TriggerMode::LandPlayed { valid_card }
            | TriggerMode::BecomesTarget { valid_card }
            | TriggerMode::TapsForMana { valid_card }
            | TriggerMode::Explored { valid_card }
            | TriggerMode::BecomeMonstrous { valid_card }
            | TriggerMode::Destroyed { valid_card }
            | TriggerMode::Exiled { valid_card }
            | TriggerMode::TokenCreated { valid_card }
            | TriggerMode::ChangesController { valid_card }
            | TriggerMode::PhasedIn { valid_card }
            | TriggerMode::PhasedOut { valid_card }
            | TriggerMode::Foretell { valid_card }
            | TriggerMode::ManaAdded { valid_card }
            | TriggerMode::TokenCreatedOnce { valid_card }
            | TriggerMode::TapAll { valid_card }
            | TriggerMode::UntapAll { valid_card }
            | TriggerMode::BecomesTargetOnce { valid_card }
            | TriggerMode::AttackerBlockedOnce { valid_card }
            | TriggerMode::AttackerUnblockedOnce { valid_card }
            | TriggerMode::DamagePreventedOnce { valid_card }
            | TriggerMode::Exerted { valid_card } => check_card_filter(
                valid_card,
                run_params.card,
                host_card,
                host_controller,
                game,
            ),

            // ── ManaExpend (check player + amount) ──
            TriggerMode::ManaExpend {
                valid_player,
                amount,
            } => {
                if !check_player_filter(valid_player, run_params.player, host_controller) {
                    return false;
                }
                // Amount must exactly match the cumulative expend amount
                run_params.mana_expend_amount == Some(*amount)
            }

            // ── Player-only triggers (check run_params.player) ──
            TriggerMode::LifeGained { valid_player }
            | TriggerMode::LifeLost { valid_player }
            | TriggerMode::BecomeMonarch { valid_player }
            | TriggerMode::TurnBegin { valid_player }
            | TriggerMode::LifeLostAll { valid_player }
            | TriggerMode::LifeGainedAll { valid_player }
            | TriggerMode::Surveil { valid_player }
            | TriggerMode::Scry { valid_player }
            | TriggerMode::SearchedLibrary { valid_player }
            | TriggerMode::Shuffled { valid_player }
            | TriggerMode::CollectEvidence { valid_player }
            | TriggerMode::Forage { valid_player } => {
                check_player_filter(valid_player, run_params.player, host_controller)
            }

            // ── AttackersDeclared (player + attacker count filtering) ──
            TriggerMode::AttackersDeclared {
                valid_player,
                valid_attackers,
                valid_attackers_amount,
            } => {
                if !check_player_filter(valid_player, run_params.player, host_controller) {
                    return false;
                }
                if valid_attackers.is_some() || valid_attackers_amount.is_some() {
                    if let Some(ref attacker_ids) = run_params.attacker_ids {
                        let matching_count = if let Some(filter) = valid_attackers {
                            attacker_ids
                                .iter()
                                .filter(|&&aid| {
                                    matches_valid_card(
                                        filter,
                                        aid,
                                        host_card,
                                        host_controller,
                                        game,
                                    )
                                })
                                .count()
                        } else {
                            attacker_ids.len()
                        };
                        if let Some(amount_filter) = valid_attackers_amount {
                            if !matches_amount(amount_filter, matching_count) {
                                return false;
                            }
                        } else if matching_count == 0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }

            // ── Parameterless triggers ──
            TriggerMode::BlockersDeclared | TriggerMode::Always | TriggerMode::Immediate => true,
        }
    }
}

fn matches_valid_sa(filter: &str, sa: &crate::spellability::SpellAbility) -> bool {
    let f = filter.trim();
    if f.is_empty() {
        return true;
    }
    if f.eq_ignore_ascii_case("Spell") {
        return sa.is_spell;
    }
    if f.eq_ignore_ascii_case("Ability") {
        return !sa.is_spell;
    }
    true
}

/// Matches a card against a ValidCard$ filter string.
/// Handles: Card.Self, Creature.Other, Creature.YouCtrl, Creature,
/// Instant,Sorcery (comma = OR), type filters.
fn matches_valid_card(
    filter: &str,
    card_id: CardId,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) -> bool {
    // Comma-separated = OR conditions
    if filter.contains(',') && !filter.contains('.') {
        return filter.split(',').any(|part| {
            matches_single_valid_card(part.trim(), card_id, host_card, host_controller, game)
        });
    }

    matches_single_valid_card(filter, card_id, host_card, host_controller, game)
}

fn matches_single_valid_card(
    filter: &str,
    card_id: CardId,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) -> bool {
    let card = game.card(card_id);

    // Split on dots for compound filters (e.g. "Creature.Other", "Card.Self")
    let parts: Vec<&str> = filter.split('.').collect();
    let type_part = parts[0];
    let qualifiers = &parts[1..];

    // Check the type portion
    let type_matches = match type_part {
        "Card" => true, // matches any card
        "Creature" => card.is_creature(),
        "Land" => card.is_land(),
        "Instant" => card.type_line.is_instant(),
        "Sorcery" => card.type_line.is_sorcery(),
        "Permanent" => card.is_permanent(),
        _ => {
            // Try comma-separated types within the type portion (e.g. "Instant,Sorcery")
            if type_part.contains(',') {
                type_part.split(',').any(|t| match t.trim() {
                    "Creature" => card.is_creature(),
                    "Land" => card.is_land(),
                    "Instant" => card.type_line.is_instant(),
                    "Sorcery" => card.type_line.is_sorcery(),
                    "Card" => true,
                    _ => false,
                })
            } else {
                true // unknown type, match all
            }
        }
    };

    if !type_matches {
        return false;
    }

    // Check qualifiers — handle compound "+" syntax (e.g. "Self+kicked", "YouCtrl+nonBlack")
    // Mirrors Java's CardProperty.isValidCard() which splits on '+' for sub-conditions.
    for &qualifier in qualifiers {
        // Split compound qualifiers on '+' (e.g. "Self+kicked" → ["Self", "kicked"])
        let sub_parts: Vec<&str> = qualifier.split('+').collect();
        for sub in &sub_parts {
            match *sub {
                "Self" => {
                    if card_id != host_card {
                        return false;
                    }
                }
                "Other" | "StrictlyOther" => {
                    if card_id == host_card {
                        return false;
                    }
                }
                "YouCtrl" => {
                    if card.controller != host_controller {
                        return false;
                    }
                }
                "OppCtrl" => {
                    if card.controller == host_controller {
                        return false;
                    }
                }
                "kicked" => {
                    if !card.kicked {
                        return false;
                    }
                }
                "nonCreature" => {
                    if card.is_creature() {
                        return false;
                    }
                }
                "nonLand" => {
                    if card.is_land() {
                        return false;
                    }
                }
                "token" => {
                    if !card.is_token {
                        return false;
                    }
                }
                "nonToken" => {
                    if card.is_token {
                        return false;
                    }
                }
                "DamagedBy" => {
                    // Check if this card was dealt damage by the host card this turn.
                    // Mirrors Java's CardProperty "DamagedBy" check using
                    // getDamageReceivedThisTurn().
                    if !card.damage_sources_this_turn.contains(&host_card) {
                        return false;
                    }
                }
                _ => {
                    // Check counters_GE/GT/LT/LE/EQ patterns like "counters_GE3_P1P1"
                    if sub.starts_with("counters_") {
                        if !check_counter_condition(sub, card) {
                            return false;
                        }
                    }
                    // Ignore unknown qualifiers for now
                }
            }
        }
    }

    true
}

/// Matches a player against a ValidPlayer$ filter string.
fn matches_valid_player(filter: &str, player: PlayerId, host_controller: PlayerId) -> bool {
    match filter {
        "You" => player == host_controller,
        "Opponent" => player != host_controller,
        "Any" | "Each" => true,
        _ => true, // unknown filter, match all
    }
}

/// Check if a count matches a ValidAttackersAmount filter like "GE1", "EQ3", etc.
fn matches_amount(filter: &str, count: usize) -> bool {
    let (op, num_str) = if filter.len() >= 3 {
        (&filter[..2], &filter[2..])
    } else {
        return count > 0; // fallback
    };
    let n: usize = num_str.parse().unwrap_or(0);
    match op {
        "GE" => count >= n,
        "GT" => count > n,
        "LE" => count <= n,
        "LT" => count < n,
        "EQ" => count == n,
        "NE" => count != n,
        _ => count > 0,
    }
}

/// Check a counter condition like "counters_GE3_P1P1".
/// Format: counters_{op}{num}_{counter_type}
fn check_counter_condition(condition: &str, card: &crate::card::CardInstance) -> bool {
    use crate::ability::effects::parse_counter_type;
    let rest = &condition["counters_".len()..];
    if rest.len() < 3 {
        return true;
    }
    let op = &rest[..2];
    let after_op = &rest[2..];
    let (num_str, counter_type_str) = match after_op.find('_') {
        Some(idx) => (&after_op[..idx], &after_op[idx + 1..]),
        None => return true,
    };
    let threshold: i32 = num_str.parse().unwrap_or(0);
    let counter_type = parse_counter_type(counter_type_str);
    let count = card.counter_count(&counter_type);
    match op {
        "GE" => count >= threshold,
        "GT" => count > threshold,
        "LE" => count <= threshold,
        "LT" => count < threshold,
        "EQ" => count == threshold,
        "NE" => count != threshold,
        _ => true,
    }
}

/// Parse a zone name to ZoneType.
fn parse_zone(s: &str) -> Option<ZoneType> {
    match s {
        "Battlefield" => Some(ZoneType::Battlefield),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" => Some(ZoneType::Library),
        "Exile" => Some(ZoneType::Exile),
        "Stack" => Some(ZoneType::Stack),
        "Command" => Some(ZoneType::Command),
        "Any" => None, // None means "any zone"
        _ => None,
    }
}

/// Parse a phase name to PhaseType.
fn parse_phase(s: &str) -> Option<PhaseType> {
    match s {
        "Untap" => Some(PhaseType::Untap),
        "Upkeep" => Some(PhaseType::Upkeep),
        "Draw" => Some(PhaseType::Draw),
        "Main1" => Some(PhaseType::Main1),
        "Main2" => Some(PhaseType::Main2),
        "CombatBegin" | "BeginCombat" => Some(PhaseType::CombatBegin),
        "CombatEnd" | "EndCombat" | "EndOfCombat" => Some(PhaseType::CombatEnd),
        "EndOfTurn" | "End" => Some(PhaseType::EndOfTurn),
        "Cleanup" => Some(PhaseType::Cleanup),
        _ => None,
    }
}

/// Mirrors the pipe-param parsing used throughout Java Forge.
/// Parses "Key1$ Value1 | Key2$ Value2" into a BTreeMap.
pub fn parse_pipe_params(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for part in raw.split('|') {
        let part = part.trim();
        if let Some(idx) = part.find("$ ") {
            let key = part[..idx].trim().to_string();
            let value = part[idx + 2..].trim().to_string();
            map.insert(key, value);
        } else if let Some(idx) = part.find('$') {
            let key = part[..idx].trim().to_string();
            let value = part[idx + 1..].trim().to_string();
            map.insert(key, value);
        }
    }
    map
}

/// Mirrors Java's TriggerHandler.parseTrigger().
/// Parses raw "Mode$ ChangesZone | Origin$ Any | ..." into Trigger struct.
pub fn parse_trigger(raw: &str, next_id: &mut u32) -> Option<Trigger> {
    let params = parse_pipe_params(raw);

    let mode_str = params.get("Mode")?;
    let mode = match mode_str.as_str() {
        "ChangesZone" => {
            let origin =
                params
                    .get("Origin")
                    .and_then(|s| if s == "Any" { None } else { parse_zone(s) });
            let destination =
                params.get("Destination").and_then(
                    |s| {
                        if s == "Any" {
                            None
                        } else {
                            parse_zone(s)
                        }
                    },
                );
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::ChangesZone {
                origin,
                destination,
                valid_card,
            }
        }
        "Phase" => {
            let phase = params.get("Phase").and_then(|s| parse_phase(s));
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Phase {
                phase,
                valid_player,
            }
        }
        "SpellCast" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCast {
                valid_card,
                valid_activating_player,
            }
        }
        "Attacks" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Attacks { valid_card }
        }
        "DamageDone" => {
            let valid_source = params.get("ValidSource").cloned();
            let valid_target = params.get("ValidTarget").cloned();
            let combat_damage_only = params
                .get("CombatDamage")
                .map(|s| s.eq_ignore_ascii_case("True"))
                .unwrap_or(false);
            TriggerMode::DamageDone {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "Countered" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_cause = params.get("ValidCause").cloned();
            let valid_sa = params.get("ValidSA").cloned();
            TriggerMode::Countered {
                valid_card,
                valid_cause,
                valid_sa,
            }
        }
        // ── New trigger modes (issue #19) ──
        "Blocks" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_blocked = params.get("ValidBlocked").cloned();
            TriggerMode::Blocks {
                valid_card,
                valid_blocked,
            }
        }
        "AttackerBlocked" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::AttackerBlocked { valid_card }
        }
        "AttackerUnblocked" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::AttackerUnblocked { valid_card }
        }
        "LifeGained" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::LifeGained { valid_player }
        }
        "LifeLost" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::LifeLost { valid_player }
        }
        "CounterAdded" => {
            let valid_card = params.get("ValidCard").cloned();
            let counter_type = params.get("CounterType").cloned();
            TriggerMode::CounterAdded {
                valid_card,
                counter_type,
            }
        }
        "CounterRemoved" => {
            let valid_card = params.get("ValidCard").cloned();
            let counter_type = params.get("CounterType").cloned();
            TriggerMode::CounterRemoved {
                valid_card,
                counter_type,
            }
        }
        "Sacrificed" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Sacrificed {
                valid_card,
                valid_player,
            }
        }
        "Drawn" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Drawn {
                valid_card,
                valid_player,
            }
        }
        "Milled" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Milled {
                valid_card,
                valid_player,
            }
        }
        "Taps" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Taps { valid_card }
        }
        "Untaps" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Untaps { valid_card }
        }
        "Transformed" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Transformed { valid_card }
        }
        "Attached" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Attached { valid_card }
        }
        "Unattached" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Unattached { valid_card }
        }
        "LandPlayed" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::LandPlayed { valid_card }
        }
        "BecomesTarget" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::BecomesTarget { valid_card }
        }
        "TapsForMana" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::TapsForMana { valid_card }
        }
        "AbilityActivated" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::AbilityActivated {
                valid_card,
                valid_activating_player,
            }
        }
        "Explored" | "Explores" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Explored { valid_card }
        }
        "BecomeMonstrous" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::BecomeMonstrous { valid_card }
        }
        "BecomeMonarch" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::BecomeMonarch { valid_player }
        }
        "DamageDealtOnce" => {
            let valid_source = params.get("ValidSource").cloned();
            let valid_target = params.get("ValidTarget").cloned();
            let combat_damage_only = params
                .get("CombatDamage")
                .map(|s| s.eq_ignore_ascii_case("True"))
                .unwrap_or(false);
            TriggerMode::DamageDealtOnce {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "Destroyed" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Destroyed { valid_card }
        }
        "Exiled" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Exiled { valid_card }
        }
        "CollectEvidence" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::CollectEvidence { valid_player }
        }
        "Forage" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Forage { valid_player }
        }
        "Enlisted" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_enlisted = params.get("ValidEnlisted").cloned();
            TriggerMode::Enlisted {
                valid_card,
                valid_enlisted,
            }
        }
        "FlippedCoin" => {
            let valid_player = params.get("ValidPlayer").cloned();
            let valid_result = params.get("ValidResult").cloned();
            TriggerMode::FlippedCoin {
                valid_player,
                valid_result,
            }
        }
        "RolledDie" => {
            let valid_player = params.get("ValidPlayer").cloned();
            let valid_result = params.get("ValidResult").cloned();
            let valid_sides = params.get("ValidSides").cloned();
            TriggerMode::RolledDie {
                valid_player,
                valid_result,
                valid_sides,
            }
        }
        "RolledDieOnce" => {
            let valid_player = params.get("ValidPlayer").cloned();
            let valid_result = params.get("ValidResult").cloned();
            let valid_sides = params.get("ValidSides").cloned();
            TriggerMode::RolledDieOnce {
                valid_player,
                valid_result,
                valid_sides,
            }
        }
        "TokenCreated" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::TokenCreated { valid_card }
        }
        // SpellCastOrCopy: used by Magecraft — fires on both cast and copy.
        // We treat it as SpellCast here; the caller can duplicate it as SpellCopied.
        "SpellCastOrCopy" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCast {
                valid_card,
                valid_activating_player,
            }
        }
        "SpellCopied" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCopied {
                valid_card,
                valid_activating_player,
            }
        }
        // ── New trigger modes (issue #54) ──
        "AttackersDeclared" => {
            let valid_player = params
                .get("AttackingPlayer")
                .or_else(|| params.get("ValidPlayer"))
                .cloned();
            let valid_attackers = params.get("ValidAttackers").cloned();
            let valid_attackers_amount = params.get("ValidAttackersAmount").cloned();
            TriggerMode::AttackersDeclared {
                valid_player,
                valid_attackers,
                valid_attackers_amount,
            }
        }
        "BlockersDeclared" => TriggerMode::BlockersDeclared,
        "ChangesZoneAll" => {
            let origin =
                params
                    .get("Origin")
                    .and_then(|s| if s == "Any" { None } else { parse_zone(s) });
            let destination =
                params
                    .get("Destination")
                    .and_then(|s| if s == "Any" { None } else { parse_zone(s) });
            let valid_card = params
                .get("ValidCards")
                .or_else(|| params.get("ValidCard"))
                .cloned();
            TriggerMode::ChangesZoneAll {
                origin,
                destination,
                valid_card,
            }
        }
        "ChangesController" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::ChangesController { valid_card }
        }
        "TurnBegin" | "NewTurn" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::TurnBegin { valid_player }
        }
        "DamageDoneOnce" => {
            let valid_source = params.get("ValidSource").cloned();
            let valid_target = params.get("ValidTarget").cloned();
            let combat_damage_only = params
                .get("CombatDamage")
                .map(|s| s.eq_ignore_ascii_case("True"))
                .unwrap_or(false);
            TriggerMode::DamageDoneOnce {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "SpellCastAll" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCastAll {
                valid_card,
                valid_activating_player,
            }
        }
        "LifeLostAll" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::LifeLostAll { valid_player }
        }
        "CounterAddedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            let counter_type = params.get("CounterType").cloned();
            let valid_source = params.get("ValidSource").cloned();
            TriggerMode::CounterAddedOnce {
                valid_card,
                counter_type,
                valid_source,
            }
        }
        "DiscardedAll" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::DiscardedAll {
                valid_card,
                valid_player,
            }
        }
        "SacrificedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::SacrificedOnce {
                valid_card,
                valid_player,
            }
        }
        "Cycled" | "Cycling" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Cycled {
                valid_card,
                valid_player,
            }
        }
        "PhasedIn" | "PhaseIn" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::PhasedIn { valid_card }
        }
        "PhasedOut" | "PhaseOut" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::PhasedOut { valid_card }
        }
        "Always" => TriggerMode::Always,
        "Immediate" => TriggerMode::Immediate,
        "Surveil" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Surveil { valid_player }
        }
        "Scry" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Scry { valid_player }
        }
        "Foretell" | "Foretold" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Foretell { valid_card }
        }
        "SearchedLibrary" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::SearchedLibrary { valid_player }
        }
        "Shuffled" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::Shuffled { valid_player }
        }
        "ManaAdded" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::ManaAdded { valid_card }
        }
        "TokenCreatedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::TokenCreatedOnce { valid_card }
        }
        "TapAll" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::TapAll { valid_card }
        }
        "UntapAll" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::UntapAll { valid_card }
        }
        "BecomesTargetOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::BecomesTargetOnce { valid_card }
        }
        "AttackerBlockedByCreature" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_blocked = params.get("ValidBlocked").cloned();
            TriggerMode::AttackerBlockedByCreature {
                valid_card,
                valid_blocked,
            }
        }
        "AttackerBlockedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::AttackerBlockedOnce { valid_card }
        }
        "AttackerUnblockedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::AttackerUnblockedOnce { valid_card }
        }
        "SpellCastOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCastOnce {
                valid_card,
                valid_activating_player,
            }
        }
        "SpellCastOfType" => {
            let valid_card = params.get("ValidCard").cloned();
            let valid_activating_player = params.get("ValidActivatingPlayer").cloned();
            TriggerMode::SpellCastOfType {
                valid_card,
                valid_activating_player,
            }
        }
        "DamageAll" => {
            let valid_source = params.get("ValidSource").cloned();
            let valid_target = params.get("ValidTarget").cloned();
            TriggerMode::DamageAll {
                valid_source,
                valid_target,
            }
        }
        "DamagePreventedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::DamagePreventedOnce { valid_card }
        }
        "ExcessDamage" => {
            let valid_source = params.get("ValidSource").cloned();
            let valid_target = params.get("ValidTarget").cloned();
            TriggerMode::ExcessDamage {
                valid_source,
                valid_target,
            }
        }
        "LifeGainedAll" => {
            let valid_player = params.get("ValidPlayer").cloned();
            TriggerMode::LifeGainedAll { valid_player }
        }
        "CounterRemovedOnce" => {
            let valid_card = params.get("ValidCard").cloned();
            let counter_type = params.get("CounterType").cloned();
            TriggerMode::CounterRemovedOnce {
                valid_card,
                counter_type,
            }
        }
        "Exerted" => {
            let valid_card = params.get("ValidCard").cloned();
            TriggerMode::Exerted { valid_card }
        }
        "ManaExpend" => {
            let valid_player = params.get("Player").cloned();
            let amount = params
                .get("Amount")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            TriggerMode::ManaExpend {
                valid_player,
                amount,
            }
        }
        _ => return None,
    };

    // Parse active zones (default: Battlefield)
    let active_zones = params
        .get("TriggerZones")
        .map(|s| {
            s.split(',')
                .filter_map(|z| parse_zone(z.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![ZoneType::Battlefield]);

    let execute = params.get("Execute").cloned().unwrap_or_default();
    let optional = params.contains_key("OptionalDecider");
    let description = params
        .get("TriggerDescription")
        .cloned()
        .unwrap_or_default();

    let id = *next_id;
    *next_id += 1;

    Some(Trigger {
        id,
        mode,
        params,
        active_zones,
        execute,
        optional,
        description,
        intrinsic: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pipe_params_basic() {
        let params = parse_pipe_params("Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw");
        assert_eq!(params.get("Mode").unwrap(), "ChangesZone");
        assert_eq!(params.get("Origin").unwrap(), "Any");
        assert_eq!(params.get("Destination").unwrap(), "Battlefield");
        assert_eq!(params.get("ValidCard").unwrap(), "Card.Self");
        assert_eq!(params.get("Execute").unwrap(), "TrigDraw");
    }

    #[test]
    fn parse_trigger_changes_zone() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When CARDNAME enters the battlefield, draw two cards.",
            &mut next_id,
        ).unwrap();

        assert_eq!(trigger.id, 0);
        assert_eq!(trigger.execute, "TrigDraw");
        assert!(matches!(
            trigger.mode,
            TriggerMode::ChangesZone {
                origin: None,
                destination: Some(ZoneType::Battlefield),
                ..
            }
        ));
        assert_eq!(trigger.active_zones, vec![ZoneType::Battlefield]);
    }

    #[test]
    fn parse_trigger_spell_cast() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | Execute$ TrigDmg | TriggerDescription$ Whenever you cast an instant or sorcery spell, deal 2 damage.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(trigger.mode, TriggerMode::SpellCast { .. }));
        assert_eq!(trigger.execute, "TrigDmg");
    }

    #[test]
    fn parse_trigger_phase() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigUpkeep | TriggerDescription$ At the beginning of your upkeep.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(
            trigger.mode,
            TriggerMode::Phase {
                phase: Some(PhaseType::Upkeep),
                ..
            }
        ));
    }

    #[test]
    fn parse_trigger_other_creature_etb() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Creature.Other | Execute$ TrigGain | TriggerDescription$ Whenever another creature enters the battlefield, gain 1 life.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(
            trigger.mode,
            TriggerMode::ChangesZone {
                origin: None,
                destination: Some(ZoneType::Battlefield),
                ..
            }
        ));

        if let TriggerMode::ChangesZone { valid_card, .. } = &trigger.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.Other"));
        }
    }

    // ── Tests for new trigger types (issue #19) ──

    #[test]
    fn parse_trigger_attacks() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Attacks | ValidCard$ Creature.Self | Execute$ TrigAtk | TriggerDescription$ When this creature attacks.",
            &mut id,
        ).unwrap();
        assert!(matches!(t.mode, TriggerMode::Attacks { .. }));
        if let TriggerMode::Attacks { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
        }
    }

    #[test]
    fn parse_trigger_damage_done() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ DamageDone | ValidSource$ Creature.Self | CombatDamage$ True | Execute$ TrigDmg",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::DamageDone {
            valid_source,
            combat_damage_only,
            ..
        } = &t.mode
        {
            assert_eq!(valid_source.as_deref(), Some("Creature.Self"));
            assert!(*combat_damage_only);
        } else {
            panic!("Expected DamageDone mode");
        }
    }

    #[test]
    fn parse_trigger_blocks() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Blocks | ValidCard$ Creature.Self | ValidBlocked$ Creature | Execute$ TrigBlock",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Blocks {
            valid_card,
            valid_blocked,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
            assert_eq!(valid_blocked.as_deref(), Some("Creature"));
        } else {
            panic!("Expected Blocks mode");
        }
    }

    #[test]
    fn parse_trigger_attacker_blocked() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AttackerBlocked | ValidCard$ Creature.Self | Execute$ TrigBlocked",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::AttackerBlocked { .. }));
    }

    #[test]
    fn parse_trigger_attacker_unblocked() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AttackerUnblocked | ValidCard$ Creature.Self | Execute$ TrigUnblocked",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::AttackerUnblocked { .. }));
    }

    #[test]
    fn parse_trigger_life_gained() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LifeGained | ValidPlayer$ You | Execute$ TrigGain | TriggerDescription$ Whenever you gain life.",
            &mut id,
        ).unwrap();
        if let TriggerMode::LifeGained { valid_player } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected LifeGained mode");
        }
    }

    #[test]
    fn parse_trigger_life_lost() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LifeLost | ValidPlayer$ Opponent | Execute$ TrigLost",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::LifeLost { valid_player } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("Opponent"));
        } else {
            panic!("Expected LifeLost mode");
        }
    }

    #[test]
    fn parse_trigger_counter_added() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ CounterAdded | ValidCard$ Card.Self | CounterType$ P1P1 | Execute$ TrigCounter",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::CounterAdded {
            valid_card,
            counter_type,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Card.Self"));
            assert_eq!(counter_type.as_deref(), Some("P1P1"));
        } else {
            panic!("Expected CounterAdded mode");
        }
    }

    #[test]
    fn parse_trigger_counter_removed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ CounterRemoved | ValidCard$ Creature | CounterType$ M1M1 | Execute$ TrigRemove",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::CounterRemoved {
            valid_card,
            counter_type,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(counter_type.as_deref(), Some("M1M1"));
        } else {
            panic!("Expected CounterRemoved mode");
        }
    }

    #[test]
    fn parse_trigger_sacrificed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Sacrificed | ValidCard$ Creature | ValidPlayer$ You | Execute$ TrigSac",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Sacrificed {
            valid_card,
            valid_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected Sacrificed mode");
        }
    }

    #[test]
    fn parse_trigger_drawn() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Drawn | ValidPlayer$ You | Execute$ TrigDraw | TriggerDescription$ Whenever you draw a card.",
            &mut id,
        ).unwrap();
        if let TriggerMode::Drawn { valid_player, .. } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected Drawn mode");
        }
    }

    #[test]
    fn parse_trigger_milled() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Milled | ValidCard$ Card | ValidPlayer$ Opponent | Execute$ TrigMill",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Milled {
            valid_card,
            valid_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Card"));
            assert_eq!(valid_player.as_deref(), Some("Opponent"));
        } else {
            panic!("Expected Milled mode");
        }
    }

    #[test]
    fn parse_trigger_taps() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Taps | ValidCard$ Creature | Execute$ TrigTap",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Taps { .. }));
    }

    #[test]
    fn parse_trigger_untaps() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Untaps | ValidCard$ Creature.Self | Execute$ TrigUntap",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Untaps { .. }));
    }

    #[test]
    fn parse_trigger_transformed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Transformed | ValidCard$ Card.Self | Execute$ TrigTransform",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Transformed { .. }));
    }

    #[test]
    fn parse_trigger_attached() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Attached | ValidCard$ Card.Self | Execute$ TrigAttach",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Attached { .. }));
    }

    #[test]
    fn parse_trigger_unattached() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Unattached | ValidCard$ Card.Self | Execute$ TrigDetach",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Unattached { .. }));
    }

    #[test]
    fn parse_trigger_land_played() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LandPlayed | ValidCard$ Land.YouCtrl | Execute$ TrigLandfall | TriggerDescription$ Landfall.",
            &mut id,
        ).unwrap();
        if let TriggerMode::LandPlayed { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Land.YouCtrl"));
        } else {
            panic!("Expected LandPlayed mode");
        }
    }

    #[test]
    fn parse_trigger_becomes_target() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ BecomesTarget | ValidCard$ Creature.Self | Execute$ TrigTarget",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::BecomesTarget { .. }));
    }

    #[test]
    fn parse_trigger_taps_for_mana() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ TapsForMana | ValidCard$ Land | Execute$ TrigMana | TriggerDescription$ Whenever a land is tapped for mana.",
            &mut id,
        ).unwrap();
        if let TriggerMode::TapsForMana { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Land"));
        } else {
            panic!("Expected TapsForMana mode");
        }
    }

    #[test]
    fn parse_trigger_ability_activated() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AbilityActivated | ValidCard$ Creature | ValidActivatingPlayer$ You | Execute$ TrigAct",
            &mut id,
        ).unwrap();
        if let TriggerMode::AbilityActivated {
            valid_card,
            valid_activating_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(valid_activating_player.as_deref(), Some("You"));
        } else {
            panic!("Expected AbilityActivated mode");
        }
    }

    #[test]
    fn parse_trigger_explored() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Explores | ValidCard$ Creature.Self | Execute$ TrigExplore",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Explored { .. }));
    }

    #[test]
    fn parse_trigger_become_monarch() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ BecomeMonarch | ValidPlayer$ You | Execute$ TrigMonarch",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::BecomeMonarch { valid_player } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected BecomeMonarch mode");
        }
    }

    #[test]
    fn parse_trigger_damage_dealt_once() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ DamageDealtOnce | ValidSource$ Creature.Self | CombatDamage$ True | Execute$ TrigOnce",
            &mut id,
        ).unwrap();
        if let TriggerMode::DamageDealtOnce {
            valid_source,
            combat_damage_only,
            ..
        } = &t.mode
        {
            assert_eq!(valid_source.as_deref(), Some("Creature.Self"));
            assert!(*combat_damage_only);
        } else {
            panic!("Expected DamageDealtOnce mode");
        }
    }

    #[test]
    fn parse_trigger_destroyed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Destroyed | ValidCard$ Creature | Execute$ TrigDestroy",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Destroyed { .. }));
    }

    #[test]
    fn parse_trigger_exiled() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Exiled | ValidCard$ Card | Execute$ TrigExile",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Exiled { .. }));
    }

    #[test]
    fn parse_trigger_token_created() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ TokenCreated | ValidCard$ Creature | Execute$ TrigToken",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::TokenCreated { .. }));
    }

    #[test]
    fn parse_trigger_optional_decider() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Creature.Other | OptionalDecider$ You | Execute$ TrigMay | TriggerDescription$ May draw.",
            &mut id,
        ).unwrap();
        assert!(t.optional);
        assert_eq!(t.description, "May draw.");
    }

    #[test]
    fn parse_trigger_countered() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Countered | ValidCard$ Card | Execute$ TrigCountered",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Countered { .. }));
    }

    #[test]
    fn parse_trigger_custom_zones() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Drawn | ValidPlayer$ You | Execute$ TrigDraw | TriggerZones$ Battlefield,Graveyard",
            &mut id,
        ).unwrap();
        assert_eq!(
            t.active_zones,
            vec![ZoneType::Battlefield, ZoneType::Graveyard]
        );
    }
}
