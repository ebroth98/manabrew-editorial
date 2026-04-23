//! Attack cost computation (Propaganda, Ghostly Prison, etc.).
//!
//! Mirrors Java Forge's `CombatUtil.getAttackCost()` — scans battlefield for
//! `CantAttackUnless` static abilities and accumulates the mana cost an
//! attacker must pay to attack a given defender.

use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::combat::DefenderId;
use crate::parsing::keys;
use crate::staticability::StaticMode;

/// Compute the total generic mana cost required for `attacker` to attack `defender`.
///
/// Returns the accumulated cost as a generic mana amount, or 0 if no cost.
/// Scans all battlefield permanents for `Mode$ CantAttackUnless` statics.
///
/// Card script example (Propaganda):
/// ```text
/// S:Mode$ CantAttackUnless | ValidCard$ Creature | Target$ You | Cost$ 2
/// ```
pub fn get_attack_cost(cards: &[Card], attacker: &Card, defender: DefenderId) -> i32 {
    let mut total_cost = 0;

    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for sa in &source.static_abilities {
            if sa.mode != StaticMode::CantAttackUnless {
                continue;
            }

            // Check ValidCard$ matches the attacker
            if !valid_filter::matches_valid_card_selector_opt(
                sa.params.selector(keys::VALID_CARD),
                attacker,
                source,
            ) {
                continue;
            }

            // Check Target$ — who is being defended
            if let Some(target_param) = sa.params.get(keys::TARGET) {
                match target_param {
                    "You" => {
                        // Only applies when attacking the source's controller
                        let defends_controller = match defender {
                            DefenderId::Player(pid) => pid == source.controller,
                            DefenderId::Permanent(cid) => {
                                cards[cid.index()].controller == source.controller
                            }
                        };
                        if !defends_controller {
                            continue;
                        }
                    }
                    "Each" | _ => {
                        // Applies to any attack
                    }
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
