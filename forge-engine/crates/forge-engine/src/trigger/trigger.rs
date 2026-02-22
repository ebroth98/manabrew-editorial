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
    },
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
            TriggerMode::ChangesZone {
                origin,
                destination,
                valid_card,
            } => {
                // Check origin zone matches
                if let Some(expected_origin) = origin {
                    if let Some(actual_origin) = run_params.origin {
                        if actual_origin != *expected_origin {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check destination zone matches
                if let Some(expected_dest) = destination {
                    if let Some(actual_dest) = run_params.destination {
                        if actual_dest != *expected_dest {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check ValidCard$ filter
                if let Some(filter) = valid_card {
                    if let Some(card_id) = run_params.card {
                        if !matches_valid_card(filter, card_id, host_card, host_controller, game) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            }

            TriggerMode::Phase {
                phase,
                valid_player,
            } => {
                // Check phase matches
                if let Some(expected_phase) = phase {
                    if let Some(actual_phase) = run_params.phase {
                        if actual_phase != *expected_phase {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check ValidPlayer$
                if let Some(filter) = valid_player {
                    if let Some(player_id) = run_params.player {
                        if !matches_valid_player(filter, player_id, host_controller) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            }

            TriggerMode::SpellCast {
                valid_card,
                valid_activating_player,
            } => {
                // Check ValidCard$ on the spell
                if let Some(filter) = valid_card {
                    if let Some(spell_card) = run_params.spell_card {
                        if !matches_valid_card(filter, spell_card, host_card, host_controller, game)
                        {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check ValidActivatingPlayer$
                if let Some(filter) = valid_activating_player {
                    if let Some(caster) = run_params.spell_controller {
                        if !matches_valid_player(filter, caster, host_controller) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            }

            TriggerMode::Attacks { valid_card } => {
                // Check ValidCard$ on attacker
                if let Some(filter) = valid_card {
                    if let Some(attacker) = run_params.attacker {
                        if !matches_valid_card(filter, attacker, host_card, host_controller, game) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            }

            TriggerMode::DamageDone {
                valid_source,
                valid_target,
                combat_damage_only,
            } => {
                // Check CombatDamage$
                if *combat_damage_only {
                    if run_params.is_combat_damage != Some(true) {
                        return false;
                    }
                }

                // Check ValidSource$
                if let Some(filter) = valid_source {
                    if let Some(source) = run_params.damage_source {
                        if !matches_valid_card(filter, source, host_card, host_controller, game) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check ValidTarget$
                if let Some(filter) = valid_target {
                    // Target can be a card or a player
                    if let Some(target_card) = run_params.damage_target_card {
                        if !matches_valid_card(
                            filter,
                            target_card,
                            host_card,
                            host_controller,
                            game,
                        ) {
                            return false;
                        }
                    } else if let Some(target_player) = run_params.damage_target_player {
                        if !matches_valid_player(filter, target_player, host_controller) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            }

            TriggerMode::Countered {
                valid_card,
                valid_cause,
            } => {
                // Check ValidCard$
                if let Some(filter) = valid_card {
                    if let Some(card_id) = run_params.card {
                        if !matches_valid_card(filter, card_id, host_card, host_controller, game) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check ValidCause$
                if let Some(_filter) = valid_cause {
                    // TODO: Implement ValidCause checking
                    // For now, assume it matches if we have a cause
                    if run_params.cause.is_none() {
                        return false;
                    }
                }

                true
            }
        }
    }
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
        return filter
            .split(',')
            .any(|part| matches_single_valid_card(part.trim(), card_id, host_card, host_controller, game));
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

    // Check qualifiers
    for &qualifier in qualifiers {
        match qualifier {
            "Self" => {
                if card_id != host_card {
                    return false;
                }
            }
            "Other" => {
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
            _ => {
                // Ignore unknown qualifiers for now
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
            let origin = params.get("Origin").and_then(|s| {
                if s == "Any" {
                    None
                } else {
                    parse_zone(s)
                }
            });
            let destination = params.get("Destination").and_then(|s| {
                if s == "Any" {
                    None
                } else {
                    parse_zone(s)
                }
            });
            let valid_card = params.get("ValidCard").map(|s| s.clone());
            TriggerMode::ChangesZone {
                origin,
                destination,
                valid_card,
            }
        }
        "Phase" => {
            let phase = params.get("Phase").and_then(|s| parse_phase(s));
            let valid_player = params.get("ValidPlayer").map(|s| s.clone());
            TriggerMode::Phase {
                phase,
                valid_player,
            }
        }
        "SpellCast" => {
            let valid_card = params.get("ValidCard").map(|s| s.clone());
            let valid_activating_player =
                params.get("ValidActivatingPlayer").map(|s| s.clone());
            TriggerMode::SpellCast {
                valid_card,
                valid_activating_player,
            }
        }
        "Attacks" => {
            let valid_card = params.get("ValidCard").map(|s| s.clone());
            TriggerMode::Attacks { valid_card }
        }
        "DamageDone" => {
            let valid_source = params.get("ValidSource").map(|s| s.clone());
            let valid_target = params.get("ValidTarget").map(|s| s.clone());
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
            let valid_card = params.get("ValidCard").map(|s| s.clone());
            let valid_cause = params.get("ValidCause").map(|s| s.clone());
            TriggerMode::Countered {
                valid_card,
                valid_cause,
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
}
