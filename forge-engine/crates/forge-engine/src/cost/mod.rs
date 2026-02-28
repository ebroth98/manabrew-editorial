use forge_foundation::{ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ability::effects::matches_change_type;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;

/// A single component of an ability cost.
/// Mirrors Java's CostPart hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostPart {
    /// Tap the source permanent. {T}
    Tap,
    /// Pay mana.
    Mana(ManaCost),
    /// Pay life.
    PayLife(i32),
    /// Sacrifice permanents. type_filter "CARDNAME" means sacrifice self.
    Sacrifice { amount: i32, type_filter: String },
    /// Discard cards. type_filter "CARDNAME" means discard self.
    Discard { amount: i32, type_filter: String },
}

impl CostPart {
    /// Payment ordering — mirrors Java's CostPart.paymentOrder.
    /// Lower numbers are paid first.
    fn payment_order(&self) -> i32 {
        match self {
            CostPart::Tap => -1,
            CostPart::Mana(_) => 0,
            CostPart::PayLife(_) => 7,
            CostPart::Sacrifice { .. } => 15,
            CostPart::Discard { .. } => 14,
        }
    }
}

/// The complete cost to activate an ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub parts: Vec<CostPart>,
    pub has_tap: bool,
}

/// Parse a Cost$ value from the DSL.
///
/// Examples:
/// - `"T"` → tap
/// - `"1 G"` → mana cost {1}{G}
/// - `"T 1 G"` → tap + mana
/// - `"Sac<1/CARDNAME>"` → sacrifice self
/// - `"PayLife<3>"` → pay 3 life
pub fn parse_cost(raw: &str) -> Cost {
    let mut parts = Vec::new();
    let mut has_tap = false;
    let mut mana_tokens: Vec<&str> = Vec::new();

    // Split on spaces, but keep <...> groups together
    let tokens = split_cost_tokens(raw);

    for token in &tokens {
        if *token == "T" {
            parts.push(CostPart::Tap);
            has_tap = true;
        } else if token.starts_with("Sac<") {
            // Parse Sac<amount/filter>
            if let Some(inner) = token.strip_prefix("Sac<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = if let Some(slash_idx) = inner.find('/') {
                    let amt = inner[..slash_idx].parse::<i32>().unwrap_or(1);
                    let filt = &inner[slash_idx + 1..];
                    (amt, filt.to_string())
                } else {
                    (1, inner.to_string())
                };
                parts.push(CostPart::Sacrifice {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("Discard<") {
            // Parse Discard<amount/filter>
            if let Some(inner) = token.strip_prefix("Discard<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = if let Some(slash_idx) = inner.find('/') {
                    let amt = inner[..slash_idx].parse::<i32>().unwrap_or(1);
                    let filt = &inner[slash_idx + 1..];
                    (amt, filt.to_string())
                } else {
                    (1, inner.to_string())
                };
                parts.push(CostPart::Discard {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("PayLife<") {
            if let Some(inner) = token
                .strip_prefix("PayLife<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let amount = inner.parse::<i32>().unwrap_or(0);
                parts.push(CostPart::PayLife(amount));
            }
        } else {
            // Accumulate as mana token
            mana_tokens.push(token);
        }
    }

    // If we have mana tokens, combine them into a ManaCost
    if !mana_tokens.is_empty() {
        let mana_str = mana_tokens.join(" ");
        let mana_cost = ManaCost::parse(&mana_str);
        if mana_cost.cmc() > 0 || !mana_str.is_empty() {
            parts.push(CostPart::Mana(mana_cost));
        }
    }

    // Sort by payment order
    parts.sort_by_key(|p| p.payment_order());

    Cost { parts, has_tap }
}

/// Split cost string on spaces, keeping `<...>` groups together.
fn split_cost_tokens(raw: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let bytes = raw.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b' ' if depth == 0 => {
                let token = raw[start..i].trim();
                if !token.is_empty() {
                    tokens.push(token);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    // Last token
    let token = raw[start..].trim();
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

/// Find valid sacrifice targets on the battlefield for a player, filtered by type.
/// Mirrors Java's `CostSacrifice.getMaxAmountX()` + `CardLists.getValidCards()`.
pub fn get_sacrifice_targets(game: &GameState, player: PlayerId, type_filter: &str) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| matches_change_type(game.card(cid), type_filter, &[]))
        .collect()
}

/// Check if a cost can be paid by the given player for the given source card.
/// `available_mana` is the total mana available (pool + untapped sources).
pub fn can_pay(
    cost: &Cost,
    game: &GameState,
    available_mana: &ManaPool,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, Some(available_mana), source, player)
}

/// Check if a cost can be paid ignoring mana requirements.
/// Used for mana ability availability checks (to avoid circular dependency).
pub fn can_pay_ignoring_mana(
    cost: &Cost,
    game: &GameState,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, None, source, player)
}

/// Shared implementation for cost payability checks.
/// When `available_mana` is None, mana costs are skipped.
fn can_pay_inner(
    cost: &Cost,
    game: &GameState,
    available_mana: Option<&ManaPool>,
    source: CardId,
    player: PlayerId,
) -> bool {
    let card = game.card(source);

    for part in &cost.parts {
        match part {
            CostPart::Tap => {
                if card.tapped {
                    return false;
                }
                if card.is_creature() && card.summoning_sick && !card.has_haste() {
                    return false;
                }
            }
            CostPart::Mana(mana_cost) => {
                if let Some(pool) = available_mana {
                    if !pool.can_pay(mana_cost) {
                        return false;
                    }
                }
            }
            CostPart::PayLife(amount) => {
                if game.player(player).life < *amount {
                    return false;
                }
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Battlefield {
                        return false;
                    }
                } else {
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            CostPart::Discard {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Hand {
                        return false;
                    }
                } else {
                    let hand_size = game.cards_in_zone(ZoneType::Hand, player).len() as i32;
                    if hand_size < *amount {
                        return false;
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tap_only() {
        let cost = parse_cost("T");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::Tap));
    }

    #[test]
    fn parse_mana_only() {
        let cost = parse_cost("1 G");
        assert!(!cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Mana(mc) => assert_eq!(mc.cmc(), 2),
            _ => panic!("expected Mana cost part"),
        }
    }

    #[test]
    fn parse_tap_and_mana() {
        let cost = parse_cost("T 1 G");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap should come first (payment_order = -1)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
    }

    #[test]
    fn parse_sacrifice() {
        let cost = parse_cost("Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "CARDNAME");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn parse_pay_life() {
        let cost = parse_cost("PayLife<3>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PayLife(n) => assert_eq!(*n, 3),
            _ => panic!("expected PayLife cost part"),
        }
    }

    #[test]
    fn parse_compound_cost() {
        let cost = parse_cost("T Sac<1/CARDNAME>");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap first (order -1), then sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Sacrifice { .. }));
    }

    #[test]
    fn parse_sacrifice_creature() {
        let cost = parse_cost("B Sac<1/Creature>");
        assert_eq!(cost.parts.len(), 2);
        // Mana (order 0) before Sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Mana(_)));
        match &cost.parts[1] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Creature");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn payment_order_sorting() {
        // PayLife, Tap, Mana, Sacrifice — should sort to: Tap, Mana, PayLife, Sacrifice
        let cost = parse_cost("PayLife<2> T 1 G Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 4);
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
        assert!(matches!(cost.parts[2], CostPart::PayLife(_)));
        assert!(matches!(cost.parts[3], CostPart::Sacrifice { .. }));
    }
}
