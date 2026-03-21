//! Block cost computation (War Cadence, etc.).
//!
//! Mirrors Java Forge's `CombatUtil.getBlockCost()` — scans battlefield for
//! `CantBlockUnless` static abilities and accumulates the mana cost a
//! blocker must pay to block a given attacker.

use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::parsing::keys;
use crate::staticability::StaticMode;

/// Compute the total generic mana cost required for `blocker` to block `attacker`.
///
/// Returns the accumulated cost as a generic mana amount, or 0 if no cost.
///
/// Card script example (War Cadence):
/// ```text
/// S:Mode$ CantBlockUnless | ValidCard$ Creature | Cost$ 1
/// ```
pub fn get_block_cost(
    cards: &[CardInstance],
    blocker: &CardInstance,
    _attacker: &CardInstance,
) -> i32 {
    let mut total_cost = 0;

    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for sa in &source.static_abilities {
            if sa.mode != StaticMode::CantBlockUnless {
                continue;
            }

            // Check ValidCard$ matches the blocker
            if let Some(valid) = sa.params.get(keys::VALID_CARD) {
                if !matches_valid_for_cost(blocker, valid) {
                    continue;
                }
            }

            // Parse Cost$ parameter as generic mana amount
            if let Some(cost_str) = sa.params.get(keys::COST) {
                if let Ok(cost) = cost_str.trim().parse::<i32>() {
                    total_cost += cost;
                }
            }
        }
    }

    total_cost
}

/// Simple valid-card check for cost statics.
fn matches_valid_for_cost(card: &CardInstance, valid: &str) -> bool {
    let parts: Vec<&str> = valid.split('.').collect();
    for part in parts {
        match part {
            "Creature" => {
                if !card.is_creature() {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}
