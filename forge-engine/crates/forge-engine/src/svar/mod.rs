//! SVar resolution system.
//!
//! SVars (Script Variables) are dynamic expressions on cards that compute values
//! at runtime based on game state. This module handles parsing and evaluating
//! SVar expressions like:
//! - `Count$Valid Forest.YouCtrl` — count matching permanents
//! - `Count$Devotion.G` — devotion to green
//! - `Count$CardPower` — power of a card
//! - `Count$Compare X GE1.3.1` — conditional expressions
//! - `X` references, `Count$xPaid`, kicked/multikicker counts, etc.
//!
//! The main entry point is `resolve_numeric_svar()` which takes a parameter name
//! from a SpellAbility and returns an integer value.

use crate::card::filter_constants as fc;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

/// Resolve a numeric parameter from a SpellAbility, expanding SVar references.
///
/// This is the main entry point for effect resolution — call it whenever you
/// need to convert a param value (which might be a literal int, "X", or an
/// SVar reference) into an integer.
///
/// **Examples:**
/// - `"NumDmg" -> "3"` → returns 3
/// - `"NumDmg" -> "X"` → returns `sa.x_mana_cost_paid` or evaluates the "X" SVar
/// - `"NumDmg" -> "AFLifeLost"` → looks up SVar "AFLifeLost" and evaluates it
///
/// **param_name**: The key in `sa.params` (e.g. "NumDmg", "LifeAmount")  
/// **default**: The value to return if the param is missing or empty  
///
/// Mirrors Java's `AbilityUtils.calculateAmount()`.
pub fn resolve_numeric_svar(
    game: &GameState,
    sa: &SpellAbility,
    param_name: &str,
    default: i32,
) -> i32 {
    let val_str = match sa.params.get(param_name) {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return default,
    };

    // Try direct integer parse first
    if let Ok(n) = val_str.trim().parse::<i32>() {
        return n;
    }
    // Try with leading + sign (e.g. "+3")
    if let Some(stripped) = val_str.trim().strip_prefix('+') {
        if let Ok(n) = stripped.parse::<i32>() {
            return n;
        }
    }

    // Check if it's the X mana cost value directly
    if val_str.trim() == "X" {
        // First check if there's an SVar named "X" on the source card
        if let Some(source_id) = sa.source {
            if let Some(svar_expr) = game.card(source_id).svars.get("X") {
                if svar_expr.starts_with("Count$") {
                    return resolve_count_svar_for_sa(
                        svar_expr,
                        game,
                        source_id,
                        sa.activating_player,
                        sa,
                    );
                }
                // TriggeredCard$CardPower / TriggeredCard$CardToughness — LKI resolution
                if svar_expr == "TriggeredCard$CardPower" {
                    if let Some(trigger_src) = sa.trigger_source {
                        return crate::lki::resolve_lki_power(game, trigger_src);
                    }
                    return 0;
                }
                if svar_expr == "TriggeredCard$CardToughness" {
                    if let Some(trigger_src) = sa.trigger_source {
                        return crate::lki::resolve_lki_toughness(game, trigger_src);
                    }
                    return 0;
                }
                return evaluate_svar(svar_expr, sa);
            }
        }
        // Otherwise use x_mana_cost_paid directly
        return sa.x_mana_cost_paid as i32;
    }

    // It's an SVar reference — look it up on the source card
    if let Some(source_id) = sa.source {
        if let Some(svar_expr) = game.card(source_id).svars.get(val_str.trim()) {
            // Game-aware SVar resolution for patterns that need GameState.
            if svar_expr.starts_with("Count$") {
                return resolve_count_svar_for_sa(
                    svar_expr,
                    game,
                    source_id,
                    sa.activating_player,
                    sa,
                );
            }
            // TriggeredCard$CardPower / TriggeredCard$CardToughness — LKI resolution
            // Mirrors Java AbilityUtils: TriggeredCard → Card, then Card$CardPower.
            if svar_expr == "TriggeredCard$CardPower" {
                if let Some(trigger_src) = sa.trigger_source {
                    return crate::lki::resolve_lki_power(game, trigger_src);
                }
                return 0;
            }
            if svar_expr == "TriggeredCard$CardToughness" {
                if let Some(trigger_src) = sa.trigger_source {
                    return crate::lki::resolve_lki_toughness(game, trigger_src);
                }
                return 0;
            }
            return evaluate_svar(svar_expr, sa);
        }
    }

    default
}

/// Evaluate a simple SVar expression.
/// Supports `Count$Kicked.A.B` (returns A if kicked, B otherwise)
/// and `Count$KickedCount` (returns the multikicker count).
pub fn evaluate_svar(expr: &str, sa: &SpellAbility) -> i32 {
    // X mana cost — return the value of X paid when casting
    if expr == "Count$xPaid" || expr == "Count$XPaid" {
        return sa.x_mana_cost_paid as i32;
    }
    // Converge/Sunburst — handled in resolve_numeric_svar (needs GameState)
    if expr == "Count$Converge" || expr == "Count$Sunburst" {
        return 0; // Fallback; game-aware path in resolve_numeric_svar handles this
    }
    if expr == "Count$TriggerRememberAmount" {
        return sa.trigger_remembered_amount;
    }
    // TriggerCount$Amount — number of objects that matched the trigger event.
    // For per-event triggers (ChangesZoneAll batched as individual fires), this is 1.
    if expr == "TriggerCount$Amount" {
        return sa.trigger_remembered_amount.max(1);
    }
    // Count$KickedCount — return the multikicker count (for Multikicker effects)
    if expr == "Count$KickedCount" {
        return sa.kick_count as i32;
    }
    // Count$Kicked.X.Y — if kicked return X, else return Y
    if let Some(rest) = expr.strip_prefix("Count$Kicked.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let kicked_val = parts[0].parse::<i32>().unwrap_or(0);
            let normal_val = parts[1].parse::<i32>().unwrap_or(0);
            return if sa.kicked { kicked_val } else { normal_val };
        }
    }
    // Number$N — literal numeric SVar (e.g. "Number$2" set by LoseLife for AFLifeLost)
    if let Some(rest) = expr.strip_prefix("Number$") {
        return rest.trim().parse::<i32>().unwrap_or(0);
    }
    // Fallback: try parsing as integer
    expr.parse::<i32>().unwrap_or(0)
}

/// Resolve a Count$ SVar expression that requires game state access.
/// Handles patterns like `Count$Valid Forest.YouCtrl`, `Count$Converge`,
/// `Count$CardPower`, etc.
/// Mirrors Java's `AbilityUtils.calculateAmount()` for Count$ expressions.
pub fn resolve_count_svar(
    expr: &str,
    game: &GameState,
    source_id: CardId,
    controller: PlayerId,
) -> i32 {
    resolve_count_svar_for_sa(
        expr,
        game,
        source_id,
        controller,
        &crate::spellability::SpellAbility::new_simple(Some(source_id), controller, ""),
    )
}

pub fn resolve_count_svar_for_sa(
    expr: &str,
    game: &GameState,
    source_id: CardId,
    controller: PlayerId,
    sa: &SpellAbility,
) -> i32 {
    use forge_foundation::ZoneType;

    if expr == "Count$TriggerRememberAmount" {
        return sa.trigger_remembered_amount;
    }

    if expr == "Count$Converge" || expr == "Count$Sunburst" {
        return game.card(source_id).sunburst_count();
    }

    if let Some(rest) = expr.strip_prefix("Count$OptionalGenericCostPaid.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let paid_val = parts[0].parse::<i32>().unwrap_or(1);
            let unpaid_val = parts[1].parse::<i32>().unwrap_or(0);
            return if sa.optional_generic_cost_paid {
                paid_val
            } else {
                unpaid_val
            };
        }
    }

    // Count$PromisedGift.A.B — return A when gift promised, else B.
    if let Some(rest) = expr.strip_prefix("Count$PromisedGift.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let promised_val = parts[0].parse::<i32>().unwrap_or(1);
            let not_promised_val = parts[1].parse::<i32>().unwrap_or(0);
            return if game.card(source_id).promised_gift.is_some() {
                promised_val
            } else {
                not_promised_val
            };
        }
    }
    if expr == "Count$PromisedGift" {
        return if game.card(source_id).promised_gift.is_some() {
            1
        } else {
            0
        };
    }

    // Count$Valid TYPE.QUALIFIERS — count permanents matching filter
    // Count$Valid TYPE.QUALIFIERS$GreatestCardPower — greatest power among matching creatures
    if let Some(filter_str) = expr.strip_prefix("Count$Valid ") {
        // Check for $GreatestCardPower suffix
        let (filter_str, greatest_power) =
            if let Some(base) = filter_str.strip_suffix("$GreatestCardPower") {
                (base, true)
            } else {
                (filter_str, false)
            };

        let battlefield = game.cards_in_zone(ZoneType::Battlefield, controller);
        // Also check opponent's battlefield for non-YouCtrl filters
        let opp = game.opponent_of(controller);
        let opp_battlefield = game.cards_in_zone(ZoneType::Battlefield, opp);

        let has_you_ctrl =
            filter_str.contains(fc::YOU_CTRL) || filter_str.contains(fc::YOU_CONTROL);

        let cards_to_check: Vec<CardId> = if has_you_ctrl {
            battlefield.to_vec()
        } else {
            battlefield
                .iter()
                .chain(opp_battlefield.iter())
                .copied()
                .collect()
        };

        let chosen_type = game.card(source_id).chosen_type.clone();
        if greatest_power {
            // Return the greatest power among matching creatures
            let mut max_power = 0;
            for &cid in &cards_to_check {
                let card = game.card(cid);
                if valid_card_matches_with_source(
                    filter_str,
                    card,
                    controller,
                    source_id,
                    chosen_type.as_deref(),
                ) {
                    max_power = max_power.max(card.power());
                }
            }
            return max_power;
        } else {
            let mut count = 0;
            for &cid in &cards_to_check {
                let card = game.card(cid);
                if valid_card_matches_with_source(
                    filter_str,
                    card,
                    controller,
                    source_id,
                    chosen_type.as_deref(),
                ) {
                    count += 1;
                }
            }
            return count;
        }
    }

    // Count$Devotion.COLOR — count mana symbols of a color among permanents you control.
    // Mirrors Java's `CardFactoryUtil.xCount()` Devotion case.
    if let Some(color_str) = expr.strip_prefix("Count$Devotion.") {
        let color_mask: u16 = match color_str.to_uppercase().as_str() {
            "W" | "WHITE" => forge_foundation::ManaAtom::WHITE,
            "U" | "BLUE" => forge_foundation::ManaAtom::BLUE,
            "B" | "BLACK" => forge_foundation::ManaAtom::BLACK,
            "R" | "RED" => forge_foundation::ManaAtom::RED,
            "G" | "GREEN" => forge_foundation::ManaAtom::GREEN,
            _ => 0,
        };
        if color_mask != 0 {
            let battlefield = game.cards_in_zone(ZoneType::Battlefield, controller);
            let mut count = 0i32;
            for &cid in battlefield {
                let card = game.card(cid);
                for shard in card.mana_cost.shards() {
                    if (shard.shard() & color_mask) != 0 {
                        count += 1;
                    }
                }
            }
            return count;
        }
    }

    // Count$Compare SVAR OPTHRESHOLD.IFTRUE.IFFALSE
    // e.g. Count$Compare Y GE1.3.1  → if Y >= 1 then 3 else 1
    if let Some(rest) = expr.strip_prefix("Count$Compare ") {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let svar_name = parts[0];
            let cond_parts: Vec<&str> = parts[1].splitn(3, '.').collect();
            if cond_parts.len() == 3 {
                // Resolve the referenced SVar
                let svar_val = if let Some(svar_expr) = game.card(source_id).svars.get(svar_name) {
                    if svar_expr.starts_with("Count$") {
                        resolve_count_svar_for_sa(svar_expr, game, source_id, controller, sa)
                    } else {
                        svar_expr.parse::<i32>().unwrap_or(0)
                    }
                } else {
                    svar_name.parse::<i32>().unwrap_or(0)
                };

                // Parse operator + threshold from cond_parts[0], e.g. "GE1"
                let cond = cond_parts[0];
                let (op, threshold) = if let Some(t) = cond.strip_prefix("GE") {
                    ("GE", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("GT") {
                    ("GT", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("LE") {
                    ("LE", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("LT") {
                    ("LT", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("EQ") {
                    ("EQ", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("NE") {
                    ("NE", t.parse::<i32>().unwrap_or(0))
                } else {
                    ("GE", 0)
                };

                let result = match op {
                    "GE" => svar_val >= threshold,
                    "GT" => svar_val > threshold,
                    "LE" => svar_val <= threshold,
                    "LT" => svar_val < threshold,
                    "EQ" => svar_val == threshold,
                    "NE" => svar_val != threshold,
                    _ => false,
                };

                let if_true = cond_parts[1].parse::<i32>().unwrap_or(0);
                let if_false = cond_parts[2].parse::<i32>().unwrap_or(0);
                return if result { if_true } else { if_false };
            }
        }
    }

    // Count$CardPower — power of the source card
    if expr == "Count$CardPower" {
        return game.card(source_id).power();
    }
    // Count$CardToughness
    if expr == "Count$CardToughness" {
        return game.card(source_id).toughness();
    }
    // Count$CardCounters.TYPE
    if let Some(counter_type) = expr.strip_prefix("Count$CardCounters.") {
        let ct = crate::ability::effects::parse_counter_type(counter_type);
        return *game.card(source_id).counters.get(&ct).unwrap_or(&0);
    }

    // Count$TotalDamageDoneByThisTurn — total damage dealt by the source card this turn.
    // Mirrors Java's Card.getTotalDamageDoneBy() via DamageHistory.
    if expr == "Count$TotalDamageDoneByThisTurn" {
        return game.card(source_id).total_damage_done_this_turn;
    }

    // Fallback
    expr.parse::<i32>().unwrap_or(1)
}

/// Check if a card matches a validity filter string like "Forest.YouCtrl".
fn valid_card_matches_with_source(
    filter: &str,
    card: &crate::card::CardInstance,
    controller: PlayerId,
    source_id: CardId,
    chosen_type: Option<&str>,
) -> bool {
    let parts: Vec<&str> = filter.split('.').collect();
    let base_type = parts.first().copied().unwrap_or("");

    // Check base type
    let type_ok = match base_type {
        fc::CREATURE => card.is_creature(),
        fc::LAND => card.is_land(),
        fc::ARTIFACT => card.type_line.is_artifact(),
        fc::ENCHANTMENT => card.type_line.is_enchantment(),
        fc::PLANESWALKER => card.type_line.is_planeswalker(),
        fc::PERMANENT | fc::CARD => true,
        // Subtypes (Forest, Island, Goblin, etc.)
        _ => card.type_line.has_subtype(base_type),
    };
    if !type_ok {
        return false;
    }

    // Check qualifiers (split by '.' and '+')
    for &dot_qual in &parts[1..] {
        for sub_qual in dot_qual.split('+') {
            let sub_qual = sub_qual.trim();
            if sub_qual.eq_ignore_ascii_case(fc::YOU_CTRL)
                || sub_qual.eq_ignore_ascii_case(fc::YOU_CONTROL)
            {
                if card.controller != controller {
                    return false;
                }
            } else if sub_qual.eq_ignore_ascii_case(fc::SELF_REF) {
                if card.id != source_id {
                    return false;
                }
            } else if sub_qual.eq_ignore_ascii_case(fc::OTHER) {
                // handled by caller if needed
            } else if sub_qual.eq_ignore_ascii_case("ChosenType") {
                // Card must have the source card's chosen creature type as a subtype.
                // Mirrors Java CardTraitBase.isValid() ChosenType qualifier.
                match chosen_type {
                    Some(ct) if card.type_line.has_subtype(ct) => {}
                    _ => return false,
                }
            } else if sub_qual.starts_with("counters_") {
                // Parse "counters_GE1_P1P1", "counters_EQ0_P1P1", etc.
                if !check_counter_qualifier(card, sub_qual) {
                    return false;
                }
            }
        }
    }
    true
}

/// Check a counter qualifier like "counters_GE1_P1P1".
fn check_counter_qualifier(card: &crate::card::CardInstance, qual: &str) -> bool {
    let rest = match qual.strip_prefix("counters_") {
        Some(r) => r,
        None => return true,
    };
    // Split into OP+THRESHOLD and COUNTER_TYPE, e.g. "GE1_P1P1"
    let parts: Vec<&str> = rest.splitn(2, '_').collect();
    if parts.len() != 2 {
        return true;
    }
    let cond = parts[0];
    let counter_type = crate::ability::effects::parse_counter_type(parts[1]);
    let count = *card.counters.get(&counter_type).unwrap_or(&0);

    let (op, threshold) = if let Some(t) = cond.strip_prefix("GE") {
        ("GE", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("GT") {
        ("GT", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("LE") {
        ("LE", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("LT") {
        ("LT", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("EQ") {
        ("EQ", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("NE") {
        ("NE", t.parse::<i32>().unwrap_or(0))
    } else {
        return true;
    };

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
