//! Effect resolution system.
//!
//! Each effect type lives in its own file, mirroring the Java Forge
//! `ability/effects/` package (204 files). Effects are dispatched by
//! API type string extracted from the ability text.

pub mod change_zone;
pub mod change_zone_all;
pub mod copy_permanent;
pub mod deal_damage;
pub mod destroy;
pub mod draw;
pub mod gain_life;
pub mod lose_life;
pub mod mana;
pub mod pump;
pub mod put_counter;
pub mod sacrifice;
pub mod sacrifice_all;
pub mod token;

use std::collections::{BTreeMap, HashMap};

use forge_foundation::ZoneType;

use crate::agent::PlayerAgent;
use crate::card::{CardInstance, CounterType};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::spellability::StackEntry;
use crate::trigger::handler::TriggerHandler;
use crate::trigger::parse_pipe_params;

/// Everything an effect needs to resolve.
pub struct EffectContext<'a> {
    pub game: &'a mut GameState,
    pub agents: &'a mut [Box<dyn PlayerAgent>],
    pub trigger_handler: &'a mut TriggerHandler,
    pub token_templates: &'a HashMap<String, CardInstance>,
    pub mana_pools: &'a mut Vec<ManaPool>,
}

/// Resolve a single effect line by detecting the API type and dispatching.
pub fn resolve_effect(ctx: &mut EffectContext, ability: &str, entry: &StackEntry) {
    let params = parse_pipe_params(ability);
    let api_type = detect_api_type(ability, &params);

    match api_type {
        "DealDamage" => deal_damage::resolve(ctx, &params, entry, ability),
        "GainLife" => gain_life::resolve(ctx, &params, entry, ability),
        "LoseLife" => lose_life::resolve(ctx, &params, entry, ability),
        "PutCounter" => put_counter::resolve(ctx, &params, entry, ability),
        "Pump" => pump::resolve(ctx, &params, entry, ability),
        "Destroy" => destroy::resolve(ctx, &params, entry),
        "Draw" => draw::resolve(ctx, &params, entry, ability),
        "ChangeZoneAll" => change_zone_all::resolve(ctx, &params, entry),
        "ChangeZone" => change_zone::resolve(ctx, &params, entry),
        "SacrificeAll" => sacrifice_all::resolve(ctx, &params, entry),
        "Sacrifice" => sacrifice::resolve(ctx, &params, entry),
        "CopyPermanent" => copy_permanent::resolve(ctx, &params, entry, ability),
        "Token" => token::resolve(ctx, &params, entry),
        "Mana" => mana::resolve(ctx, &params, entry),
        _ => {} // Unimplemented effect — silently skip
    }
}

/// Detect the API type from an ability string.
///
/// Tries structured detection first (SP$, DB$, AB$ prefix), then falls
/// back to contains-matching for compatibility with existing card scripts.
fn detect_api_type<'a>(ability: &'a str, params: &'a BTreeMap<String, String>) -> &'a str {
    // Structured: check for SP$, DB$, AB$ keys in the parsed params
    for key in &["SP", "DB", "AB"] {
        if let Some(val) = params.get(*key) {
            return val.as_str();
        }
    }

    // Fallback: contains-matching (order matters — check longer names first)
    // ChangeZoneAll must be checked before ChangeZone, SacrificeAll before Sacrifice
    if ability.contains("DealDamage") {
        "DealDamage"
    } else if ability.contains("GainLife") {
        "GainLife"
    } else if ability.contains("LoseLife") {
        "LoseLife"
    } else if ability.contains("PutCounter") {
        "PutCounter"
    } else if ability.contains("$ Pump") {
        "Pump"
    } else if ability.contains("CopyPermanent") {
        "CopyPermanent"
    } else if ability.contains("Destroy") {
        "Destroy"
    } else if ability.contains("Draw") {
        "Draw"
    } else if ability.contains("ChangeZoneAll") {
        "ChangeZoneAll"
    } else if ability.contains("ChangeZone") {
        "ChangeZone"
    } else if ability.contains("SacrificeAll") {
        "SacrificeAll"
    } else if ability.contains("Sacrifice") {
        "Sacrifice"
    } else if ability.contains("Token") {
        "Token"
    } else if ability.contains("Mana") {
        "Mana"
    } else {
        ""
    }
}

// ── Shared helpers used by multiple effects ───────────────────────────

/// Parse a numeric parameter from an ability string (e.g. "NumAtt$ 3" → 3).
pub fn parse_param(ability: &str, prefix: &str) -> Option<i32> {
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix(prefix) {
            if let Ok(n) = val.trim().parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Parse NumDmg$ value from an ability string.
pub fn parse_num_dmg(ability: &str) -> i32 {
    parse_param(ability, "NumDmg$ ").unwrap_or(0)
}

/// Resolve a Defined$ parameter to a player ID.
/// Mirrors Java's AbilityUtils.getDefinedPlayers().
pub fn resolve_defined_player(
    defined: &str,
    controller: PlayerId,
    game: &GameState,
) -> Option<PlayerId> {
    match defined {
        "You" => Some(controller),
        "Opponent" | "OpponentCtrl" => {
            let opp = game.opponent_of(controller);
            Some(opp)
        }
        _ => None,
    }
}

/// Parse a counter type string to CounterType enum.
pub fn parse_counter_type(s: &str) -> CounterType {
    match s {
        "P1P1" | "+1/+1" => CounterType::P1P1,
        "M1M1" | "-1/-1" => CounterType::M1M1,
        "Loyalty" => CounterType::Loyalty,
        "Charge" => CounterType::Charge,
        _ => CounterType::P1P1,
    }
}

/// Parse a zone name string to ZoneType.
pub fn parse_zone_type(s: &str) -> Option<ZoneType> {
    match s.trim() {
        "Battlefield" => Some(ZoneType::Battlefield),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" | "Deck" => Some(ZoneType::Library),
        "Exile" => Some(ZoneType::Exile),
        "Command" => Some(ZoneType::Command),
        _ => None,
    }
}

/// Convert a Produced$ value (e.g. "G", "R", "W") to a ManaAtom.
/// Re-exported from the mana module for convenience in effect files.
pub use crate::mana::mana_atom_from_produced;

/// Check if a card matches a ChangeType$ / ValidCards$ filter string.
pub fn matches_change_type(card: &CardInstance, change_type: &str) -> bool {
    if change_type.is_empty() {
        return true;
    }

    let parts: Vec<&str> = change_type.split('.').collect();
    let type_part = parts[0];

    let type_matches = match type_part {
        "Land" => card.is_land(),
        "Creature" => card.is_creature(),
        "Card" => true,
        _ => true,
    };

    if !type_matches {
        return false;
    }

    for &qualifier in &parts[1..] {
        match qualifier {
            "Basic" => {
                if !card.type_line.is_basic() {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

/// Emit a ChangesZone trigger event. Used by multiple zone-moving effects.
pub fn emit_zone_trigger(
    trigger_handler: &mut TriggerHandler,
    card_id: CardId,
    origin: ZoneType,
    destination: ZoneType,
) {
    trigger_handler.run_trigger(
        TriggerType::ChangesZone,
        RunParams {
            card: Some(card_id),
            origin: Some(origin),
            destination: Some(destination),
            ..Default::default()
        },
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_num_dmg_test() {
        assert_eq!(
            parse_num_dmg(
                "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ test"
            ),
            3
        );
    }

    #[test]
    fn parse_param_test() {
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumAtt$ "),
            Some(3)
        );
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumDef$ "),
            Some(3)
        );
        assert_eq!(
            parse_param("SP$ Draw | NumCards$ 2", "NumCards$ "),
            Some(2)
        );
    }

    #[test]
    fn detect_api_type_sp_prefix() {
        let params = parse_pipe_params("SP$ DealDamage | NumDmg$ 3");
        assert_eq!(detect_api_type("SP$ DealDamage | NumDmg$ 3", &params), "DealDamage");
    }

    #[test]
    fn detect_api_type_db_prefix() {
        let params = parse_pipe_params("DB$ Draw | NumCards$ 2");
        assert_eq!(detect_api_type("DB$ Draw | NumCards$ 2", &params), "Draw");
    }

    #[test]
    fn detect_api_type_fallback() {
        let params = BTreeMap::new();
        assert_eq!(detect_api_type("something with ChangeZoneAll", &params), "ChangeZoneAll");
        assert_eq!(detect_api_type("something with ChangeZone", &params), "ChangeZone");
        assert_eq!(detect_api_type("something with SacrificeAll", &params), "SacrificeAll");
        assert_eq!(detect_api_type("something with Sacrifice", &params), "Sacrifice");
    }
}
