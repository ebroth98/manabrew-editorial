//! Effect resolution system.
//!
//! Each effect type lives in its own file, mirroring the Java Forge
//! `ability/effects/` package (204 files). Effects are dispatched by
//! API type string extracted from the ability text.

pub mod change_zone_effect;
pub mod change_zone_all_effect;
pub mod copy_permanent_effect;
pub mod damage_deal_effect;
pub mod destroy_effect;
pub mod dig_effect;
pub mod dig_multiple_effect;
pub mod draw_effect;
pub mod life_gain_effect;
pub mod look_at_effect;
pub mod life_lose_effect;
pub mod mana_effect;
pub mod mill_effect;
pub mod pump_effect;
pub mod counters_put_effect;
pub mod rearrange_top_of_library_effect;
pub mod reveal_effect;
pub mod reveal_hand_effect;
pub mod sacrifice_effect;
pub mod sacrifice_all_effect;
pub mod scry_effect;
pub mod surveil_effect;
pub mod token_effect;

use std::collections::HashMap;

use forge_foundation::ZoneType;

use crate::agent::PlayerAgent;
use crate::card::{CardInstance, CounterType};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::spellability::SpellAbility;
use crate::trigger::handler::TriggerHandler;

/// Everything an effect needs to resolve.
pub struct EffectContext<'a> {
    pub game: &'a mut GameState,
    pub agents: &'a mut [Box<dyn PlayerAgent>],
    pub trigger_handler: &'a mut TriggerHandler,
    pub token_templates: &'a HashMap<String, CardInstance>,
    pub mana_pools: &'a mut Vec<ManaPool>,
}

/// Resolve a single SpellAbility node's effect by dispatching on its API type.
/// Mirrors Java's `AbilityUtils.resolveApiAbility(sa)`.
pub fn resolve_effect(ctx: &mut EffectContext, sa: &SpellAbility) {
    let api_type = sa.api.as_deref().unwrap_or_else(|| {
        // Fallback: detect from ability text (for backwards compat)
        detect_api_type_from_text(&sa.ability_text)
    });

    match api_type {
        "DealDamage" => damage_deal_effect::resolve(ctx, sa),
        "GainLife" => life_gain_effect::resolve(ctx, sa),
        "LoseLife" => life_lose_effect::resolve(ctx, sa),
        "PutCounter" => counters_put_effect::resolve(ctx, sa),
        "Pump" => pump_effect::resolve(ctx, sa),
        "Destroy" => destroy_effect::resolve(ctx, sa),
        "Draw" => draw_effect::resolve(ctx, sa),
        "ChangeZoneAll" => change_zone_all_effect::resolve(ctx, sa),
        "ChangeZone" => change_zone_effect::resolve(ctx, sa),
        "SacrificeAll" => sacrifice_all_effect::resolve(ctx, sa),
        "Sacrifice" => sacrifice_effect::resolve(ctx, sa),
        "CopyPermanent" => copy_permanent_effect::resolve(ctx, sa),
        "Token" => token_effect::resolve(ctx, sa),
        "Mana" => mana_effect::resolve(ctx, sa),
        // Library manipulation (issue #15)
        "Mill" => mill_effect::resolve(ctx, sa),
        "Scry" => scry_effect::resolve(ctx, sa),
        "Surveil" => surveil_effect::resolve(ctx, sa),
        "Dig" => dig_effect::resolve(ctx, sa),
        "DigMultiple" => dig_multiple_effect::resolve(ctx, sa),
        "RearrangeTopOfLibrary" => rearrange_top_of_library_effect::resolve(ctx, sa),
        // Reveal / Look (informational)
        "Reveal" => reveal_effect::resolve(ctx, sa),
        "RevealHand" => reveal_hand_effect::resolve(ctx, sa),
        "LookAt" => look_at_effect::resolve(ctx, sa),
        _ => {} // Unimplemented effect — silently skip
    }
}

/// Fallback: detect API type from raw ability text via contains-matching.
/// Only used when `SpellAbility.api` is None (shouldn't happen for properly
/// parsed abilities, but kept for backward compatibility).
fn detect_api_type_from_text(ability: &str) -> &'static str {
    // Order matters — check longer names first
    // ChangeZoneAll must be checked before ChangeZone, SacrificeAll before Sacrifice
    // RevealHand before Reveal, DigMultiple before Dig
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
    } else if ability.contains("Mill") {
        "Mill"
    } else if ability.contains("Scry") {
        "Scry"
    } else if ability.contains("Surveil") {
        "Surveil"
    } else if ability.contains("DigMultiple") {
        "DigMultiple"
    } else if ability.contains("$ Dig") {
        "Dig"
    } else if ability.contains("RearrangeTopOfLibrary") {
        "RearrangeTopOfLibrary"
    } else if ability.contains("RevealHand") {
        "RevealHand"
    } else if ability.contains("Reveal") {
        "Reveal"
    } else if ability.contains("LookAt") {
        "LookAt"
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
    fn detect_api_type_fallback() {
        assert_eq!(detect_api_type_from_text("something with ChangeZoneAll"), "ChangeZoneAll");
        assert_eq!(detect_api_type_from_text("something with ChangeZone"), "ChangeZone");
        assert_eq!(detect_api_type_from_text("something with SacrificeAll"), "SacrificeAll");
        assert_eq!(detect_api_type_from_text("something with Sacrifice"), "Sacrifice");
    }
}
