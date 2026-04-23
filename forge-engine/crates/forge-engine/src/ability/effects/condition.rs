//! SpellAbility condition gating.
//!
//! Mirrors Java's `SpellAbility.checkConditions()` +
//! `SpellAbilityCondition.checkConditions()` for the `Condition$`,
//! `ConditionCheckSVar$`, `ConditionPresent$`, `ConditionZone$`,
//! `ConditionCompare$`, `ConditionDefined$` params.

use forge_foundation::ZoneType;

use crate::card::Card;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

use super::helpers::matches_valid_cards_for_sa;

/// Check if a conditional gate on this SA is satisfied.
/// Mirrors Java's `SpellAbility.checkConditions()` +
/// `SpellAbilityCondition.areMet()` for the common gate types.
pub(super) fn check_condition(game: &GameState, sa: &SpellAbility) -> bool {
    let activator = sa.activating_player;

    // Player-state gates (SpellAbilityCondition.areMet L263–L269).
    if sa.params.is_true("ConditionHellbent") && !crate::player::has_hellbent(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionThreshold") && !crate::player::has_threshold(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionMetalcraft") && !crate::player::has_metalcraft(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionDelirium") && !crate::player::has_delirium(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionRevolt") && !crate::player::has_revolt(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionDesert") && !crate::player::has_desert(game, activator) {
        return false;
    }
    if sa.params.is_true("ConditionBlessing") && !crate::player::has_blessing(game, activator) {
        return false;
    }

    // Kicked-related flags (L271–L280).
    if sa.params.is_true("ConditionKicked") && !sa.kicked {
        return false;
    }
    if sa.params.is_true("ConditionOptionalPaid") && !sa.optional_generic_cost_paid {
        return false;
    }
    if sa.params.is_true("ConditionOptionalNotPaid") && sa.optional_generic_cost_paid {
        return false;
    }

    // Turn-owner gates (L300–L311).
    if let Some(raw) = sa.params.get("ConditionPlayerTurn") {
        let expect_self_turn = !raw.eq_ignore_ascii_case("False");
        let is_self_turn = game.turn.active_player == activator;
        if expect_self_turn != is_self_turn {
            return false;
        }
    }
    if sa.params.is_true("ConditionOpponentTurn") && game.turn.active_player == activator {
        return false;
    }

    // Hand-size gate: `ConditionCardsInHand$ N` or `ConditionCardsInHand$ GE3`.
    if let Some(raw) = sa.params.get("ConditionCardsInHand") {
        let size = game
            .cards_in_zone(forge_foundation::ZoneType::Hand, activator)
            .len() as i32;
        if let Ok(n) = raw.parse::<i32>() {
            if size != n {
                return false;
            }
        } else if !compare_expr(size, raw) {
            return false;
        }
    }

    // Phase gate: `ConditionPhases$ End Of Turn,Upkeep` (comma-separated).
    if let Some(phases) = sa.params.get("ConditionPhases") {
        let current = game.turn.phase;
        let ok = phases
            .split(',')
            .map(str::trim)
            .filter_map(forge_foundation::PhaseType::from_script_name)
            .any(|p| p == current);
        if !ok {
            return false;
        }
    }

    // Life-compare gate: `ConditionLifeCompare$ GE20`.
    if let Some(cmp) = sa.params.get("ConditionLifeCompare") {
        if !compare_expr(game.player(activator).life, cmp) {
            return false;
        }
    }

    // Check Condition$ Kicked (most common pattern: simple kicked gate)
    if let Some(cond) = sa.params.get(keys::CONDITION) {
        if cond == "Kicked" {
            return sa.kicked;
        }
    }
    // Check ConditionCheckSVar$ Kicked (SVar-based kicked gate)
    if let Some(cond) = sa.params.get(keys::CONDITION_CHECK_SVAR) {
        if cond == "Kicked" || cond == "X:Kicked" {
            return sa.kicked;
        }
        let compare = sa.params.get("ConditionSVarCompare").unwrap_or("GE1");
        let Some(source_id) = sa.source else {
            return false;
        };
        let Some(expr) = game.card(source_id).get_s_var(cond) else {
            return false;
        };

        let value = if let Some(valid_filter) = expr.strip_prefix("Imprinted$Valid ") {
            let imprinted = game.card(source_id).imprinted_cards.clone();
            if valid_filter.eq_ignore_ascii_case("Card.sharesNameWith Remembered") {
                let remembered_names: std::collections::HashSet<String> = game
                    .card(source_id)
                    .remembered_cards
                    .iter()
                    .map(|&cid| game.card(cid).card_name.clone())
                    .collect();
                imprinted
                    .into_iter()
                    .filter(|&cid| remembered_names.contains(&game.card(cid).card_name))
                    .count() as i32
            } else {
                imprinted
                    .into_iter()
                    .filter(|&cid| {
                        matches_valid_cards_for_sa(game, sa, game.card(cid), None, valid_filter)
                    })
                    .count() as i32
            }
        } else {
            crate::svar::resolve_count_svar_for_sa(expr, game, source_id, sa.activating_player, sa)
        };
        return compare_expr(value, compare);
    }
    true
}

/// Check ConditionPresent$ / ConditionZone$ / ConditionCompare$ against game state.
/// Returns true if the condition is met (or if no condition params exist).
///
/// When `ConditionDefined$` is present, check the defined cards instead of
/// scanning a zone.  Mirrors Java's `SpellAbilityCondition.checkConditions()`.
pub(super) fn check_condition_present(
    game: &GameState,
    sa: &SpellAbility,
    player: PlayerId,
    source_id: CardId,
) -> bool {
    let condition = match sa.params.get_cloned(keys::CONDITION_PRESENT) {
        Some(c) => c,
        None => return true, // No condition — always passes
    };

    // Parse condition alternatives (comma-separated)
    let alternatives: Vec<&str> = condition.split(',').map(|s| s.trim()).collect();

    // ── ConditionDefined$ — check specific defined cards, not a zone ──
    if let Some(cond_defined) = sa.params.get(keys::CONDITION_DEFINED) {
        let defined_cards: Vec<CardId> = match cond_defined {
            "Targeted" => sa.target_chosen.target_card.into_iter().collect(),
            "Self" => sa.source.into_iter().collect(),
            "Remembered" => sa
                .source
                .map(|sid| game.card(sid).remembered_cards.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        };

        // ConditionDefined$ cards are explicitly defined — don't exclude self.
        // Self-exclusion only makes sense for the zone-scan path below.
        let count = defined_cards
            .iter()
            .filter(|&&cid| {
                matches_condition_filter_no_self_exclude(
                    game,
                    cid,
                    source_id,
                    player,
                    &alternatives,
                )
            })
            .count() as i32;

        return if let Some(compare) = sa.params.get(keys::CONDITION_COMPARE) {
            compare_expr(count, compare)
        } else {
            count > 0
        };
    }

    let zone_str = sa.params.get(keys::CONDITION_ZONE).unwrap_or("Battlefield");

    let zone = match zone_str.to_ascii_lowercase().as_str() {
        "graveyard" => ZoneType::Graveyard,
        "hand" => ZoneType::Hand,
        "exile" => ZoneType::Exile,
        "library" => ZoneType::Library,
        _ => ZoneType::Battlefield,
    };

    // Count matching cards in zone
    let cards = game.cards_in_zone(zone, player);
    let count = cards
        .iter()
        .filter(|&&cid| matches_condition_filter(game, cid, source_id, player, &alternatives))
        .count() as i32;

    // Check ConditionCompare$ (e.g. "GE2", "EQ0")
    if let Some(compare) = sa.params.get(keys::CONDITION_COMPARE) {
        compare_expr(count, compare)
    } else {
        count > 0
    }
}

/// Check if a card matches a condition filter expression.
/// Handles type matching + qualifier checks (YouCtrl, OppCtrl, ChosenCtrl, etc.).
/// Like matches_condition_filter but without self-exclusion.
/// Used by ConditionDefined$ where the defined cards are explicitly specified.
fn matches_condition_filter_no_self_exclude(
    game: &GameState,
    cid: CardId,
    source_id: CardId,
    player: PlayerId,
    alternatives: &[&str],
) -> bool {
    let card = game.card(cid);
    let source = game.card(source_id);
    alternatives.iter().any(|alt| {
        let (base, qualifier) = if let Some((b, q)) = alt.split_once('.') {
            (b, Some(q))
        } else {
            (*alt, None)
        };
        let type_ok = match base.to_ascii_lowercase().as_str() {
            "card" => true,
            "creature" => card.is_creature(),
            "instant" => card.type_line.is_instant(),
            "sorcery" => card.type_line.is_sorcery(),
            "artifact" => card.type_line.is_artifact(),
            "enchantment" => card.type_line.is_enchantment(),
            "land" => card.is_land(),
            "planeswalker" => card.type_line.is_planeswalker(),
            _ => card.type_line.has_subtype(base),
        };
        if !type_ok {
            return false;
        }
        if let Some(q) = qualifier {
            match q.to_ascii_lowercase().as_str() {
                "basic" => card.type_line.is_basic(),
                "nonbasic" => !card.type_line.is_basic(),
                "youctrl" | "youown" => card.controller == player,
                "oppctrl" => card.controller != player,
                "chosenctrl" => source
                    .chosen_player
                    .is_some_and(|chosen| card.controller == chosen),
                _ => true,
            }
        } else {
            true
        }
    })
}

fn matches_condition_filter(
    game: &GameState,
    cid: CardId,
    source_id: CardId,
    player: PlayerId,
    alternatives: &[&str],
) -> bool {
    if cid == source_id {
        return false; // Don't count self
    }
    let card = game.card(cid);
    let source = game.card(source_id);
    alternatives
        .iter()
        .any(|alt| filter_expr_matches(card, source, player, alt))
}

/// Evaluate a single `Type.Qualifier1+Qualifier2+...` filter expression.
/// All qualifiers must pass (AND semantics, mirroring Java's CardType.matches).
fn filter_expr_matches(card: &Card, source: &Card, player: PlayerId, alt: &str) -> bool {
    let (base, qualifiers) = if let Some((b, q)) = alt.split_once('.') {
        (b, Some(q))
    } else {
        (alt, None)
    };
    let type_ok = match base.to_ascii_lowercase().as_str() {
        "card" | "permanent" => true,
        "creature" => card.is_creature(),
        "instant" => card.type_line.is_instant(),
        "sorcery" => card.type_line.is_sorcery(),
        "artifact" => card.type_line.is_artifact(),
        "enchantment" => card.type_line.is_enchantment(),
        "land" => card.is_land(),
        "planeswalker" => card.type_line.is_planeswalker(),
        _ => card.type_line.has_subtype(base),
    };
    if !type_ok {
        return false;
    }
    let Some(qualifiers) = qualifiers else {
        return true;
    };
    // Qualifiers are '+'-separated and must all match (AND).
    qualifiers
        .split('+')
        .all(|q| match q.to_ascii_lowercase().as_str() {
            "basic" => card.type_line.is_basic(),
            "nonbasic" => !card.type_line.is_basic(),
            "youctrl" | "youown" => card.controller == player,
            "oppctrl" | "oppown" => card.controller != player,
            "chosenctrl" => source
                .chosen_player
                .map_or(false, |chosen| card.controller == chosen),
            "" => true,
            // Unknown qualifier — permissive (matches Java fallthrough).
            _ => true,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::spellability::SpellAbility;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    #[test]
    fn condition_present_targeted_nonbasic_matches_destroyed_land() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1"], 20);
        let spell_source = game.create_card(Card::new(
            CardId(0),
            "Choking Sands".to_string(),
            player,
            CardTypeLine::parse("Sorcery"),
            ManaCost::parse("1 B B"),
            ColorSet::BLACK,
            None,
            None,
            vec![],
            vec![],
        ));
        let target = game.create_card(Card::new(
            CardId(0),
            "Cliffgate".to_string(),
            player,
            CardTypeLine::parse("Land - Gate"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        ));

        let mut sa = SpellAbility::new_simple(Some(spell_source), player, "DB$ DealDamage");
        sa.params
            .put(keys::CONDITION_DEFINED.to_string(), "Targeted".to_string());
        sa.params.put(
            keys::CONDITION_PRESENT.to_string(),
            "Land.Basic".to_string(),
        );
        sa.params
            .put(keys::CONDITION_COMPARE.to_string(), "EQ0".to_string());
        sa.target_chosen.target_card = Some(target);

        assert!(check_condition_present(&game, &sa, player, spell_source));
    }
}
