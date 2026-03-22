//! Cost adjustment logic for spells and abilities.
//!
//! Mirrors Java's `forge.game.cost.CostAdjustment`.
//!
//! Scans static abilities on the battlefield (and the spell's own card) to
//! compute mana cost reductions, increases, set-cost floors (Trinisphere),
//! and additional non-mana cost parts (e.g. sacrifice from `Cost$` params).

use forge_foundation::color::Color;
use forge_foundation::mana::ManaCost;
use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::cost::{parse_cost, Cost};
use crate::game::GameState;
use crate::parsing::keys;
use crate::ids::{CardId, PlayerId};
use crate::staticability::StaticMode;

// ── CostAdjustment result struct ─────────────────────────────────────

/// Result of computing cost adjustments from static abilities.
///
/// Mirrors the net effect of Java's `CostAdjustment.adjust(ManaCostBeingPaid, ...)`
/// after scanning all ReduceCost / IncreaseCost / SetCost static abilities.
#[derive(Debug, Clone, Default)]
pub struct CostAdjustment {
    /// Generic mana adjustment (positive = increase, negative = reduction).
    pub generic: i32,
    /// Per-color reductions: (color, amount, ignore_generic).
    pub color_reductions: Vec<(Color, i32, bool)>,
    /// Per-color increases: (color, amount).
    pub color_increases: Vec<(Color, i32)>,
    /// Minimum total mana cost after reductions (from MinMana$).
    pub min_mana: Option<i32>,
    /// Raise-to minimum (from SetCost + RaiseTo$, e.g. Trinisphere).
    pub raise_to: Option<i32>,
}

impl CostAdjustment {
    pub fn is_empty(&self) -> bool {
        self.generic == 0
            && self.color_reductions.is_empty()
            && self.color_increases.is_empty()
            && self.min_mana.is_none()
            && self.raise_to.is_none()
    }

    /// Apply this adjustment to a ManaCost, returning the modified cost.
    ///
    /// Mirrors the combined effect of Java's `applyReduceCostAbility`,
    /// `applySetCostAbility`, and generic increase logic in `CostAdjustment.adjust`.
    pub fn apply(&self, cost: &ManaCost) -> ManaCost {
        let mut result = cost.clone();

        // Apply color-specific reductions
        for &(color, amount, ignore_generic) in &self.color_reductions {
            result = result.reduce_color(color, amount, ignore_generic);
        }

        // Apply color-specific increases
        for &(color, amount) in &self.color_increases {
            let shard = color_to_shard(color);
            for _ in 0..amount {
                let mut shards = result.shards().to_vec();
                shards.push(shard);
                result = ManaCost::from_parts(shards, result.generic_cost());
            }
        }

        // Apply generic adjustment
        if self.generic > 0 {
            result = result.add(&ManaCost::generic(self.generic));
        } else if self.generic < 0 {
            result = result.reduce_generic(-self.generic);
        }

        // Enforce MinMana$ floor
        if let Some(min) = self.min_mana {
            if result.cmc() < min {
                let deficit = min - result.cmc();
                if deficit > 0 {
                    result = result.add(&ManaCost::generic(deficit));
                }
            }
        }

        // Enforce RaiseTo$ (Trinisphere): if cost would be less than N, raise to N
        if let Some(raise) = self.raise_to {
            if result.cmc() < raise {
                let deficit = raise - result.cmc();
                if deficit > 0 {
                    result = result.add(&ManaCost::generic(deficit));
                }
            }
        }

        result
    }

    /// Return the net generic change for simple affordability checks.
    pub fn net_generic_estimate(&self) -> i32 {
        let mut net = self.generic;
        for &(_, amount, _) in &self.color_reductions {
            net -= amount;
        }
        for &(_, amount) in &self.color_increases {
            net += amount;
        }
        net
    }
}

fn color_to_shard(color: Color) -> forge_foundation::mana::ManaCostShard {
    use forge_foundation::mana::ManaCostShard;
    match color {
        Color::White => ManaCostShard::White,
        Color::Blue => ManaCostShard::Blue,
        Color::Black => ManaCostShard::Black,
        Color::Red => ManaCostShard::Red,
        Color::Green => ManaCostShard::Green,
    }
}

// ── Public API: compute_cost_adjustment ──────────────────────────────

/// Compute cost adjustments for casting `spell_card` by `caster` from `cast_zone`.
///
/// Scans all battlefield permanents for ReduceCost / IncreaseCost / SetCost static abilities.
/// Mirrors Java's `CostAdjustment.adjust(ManaCostBeingPaid, ...)` scanning loop.
pub fn compute_cost_adjustment(
    game: &GameState,
    spell_card: &Card,
    caster: PlayerId,
    cast_zone: ZoneType,
) -> CostAdjustment {
    compute_cost_adjustment_with_targets(game, spell_card, caster, cast_zone, &[])
}

/// Like `compute_cost_adjustment`, but also checks ValidTarget$ against chosen targets.
pub fn compute_cost_adjustment_with_targets(
    game: &GameState,
    spell_card: &Card,
    caster: PlayerId,
    cast_zone: ZoneType,
    targets: &[CardId],
) -> CostAdjustment {
    let mut adj = CostAdjustment::default();

    for source in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.id == spell_card.id)
    {
        for st_ab in source.static_abilities.iter() {
            let is_reduce;
            let is_set_cost;
            match st_ab.mode {
                StaticMode::ReduceCost => {
                    is_reduce = true;
                    is_set_cost = false;
                }
                StaticMode::IncreaseCost => {
                    is_reduce = false;
                    is_set_cost = false;
                }
                StaticMode::SetCost => {
                    is_reduce = false;
                    is_set_cost = true;
                }
                _ => continue,
            };

            // Java `CostAdjustment.applyRaiseCostAbility` merges `Cost$...` directly
            // into the spell's payable cost, not into generic mana deltas.
            // Those parts are handled by `compute_raise_cost_parts`; skip here.
            if st_ab.params.has(keys::COST) {
                continue;
            }

            // ── checkRequirement: Type$ filter ───────────────────────
            if let Some(type_filter) = st_ab.params.get(keys::TYPE) {
                match type_filter.to_ascii_lowercase().as_str() {
                    "spell" => { /* casting a spell — ok */ }
                    _ => continue,
                }
            }

            // ── checkRequirement: Activator$ ─────────────────────────
            if let Some(activator) = st_ab.params.get(keys::ACTIVATOR) {
                match activator.to_ascii_lowercase().as_str() {
                    "you" => {
                        if source.controller != caster {
                            continue;
                        }
                    }
                    "opponent" => {
                        if source.controller == caster {
                            continue;
                        }
                    }
                    _ => {
                        eprintln!("[WARN] Unknown cost adjustment Activator: {:?}", activator);
                    }
                }
            } else {
                // Default for ReduceCost/SetCost: applies to controller only.
                // Default for IncreaseCost: applies to ALL players (e.g. Thalia,
                // Guardian of Thraben — "Noncreature spells cost {1} more to cast").
                if !is_reduce && !is_set_cost {
                    // IncreaseCost without Activator$ → universal effect
                } else if source.controller != caster {
                    continue;
                }
            }

            // ── checkRequirement: ValidCard$ ─────────────────────────
            if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
                if !matches_valid_card(valid_card, spell_card, source) {
                    continue;
                }
            }

            // ── checkRequirement: EffectZone$ / AffectedZone$ ────────
            if let Some(zone_str) = st_ab
                .params
                .get("EffectZone")
                .or_else(|| st_ab.params.get(keys::AFFECTED_ZONE))
            {
                if !zone_str.eq_ignore_ascii_case("All") {
                    let zones: Vec<&str> = zone_str.split(',').map(|s| s.trim()).collect();
                    if !zones.iter().any(|z| zone_name_matches(cast_zone, z)) {
                        continue;
                    }
                }
            }

            // ── checkRequirement: IsPresent$ / PresentZone$ ──────────
            if !valid_filter::check_is_present(game, &st_ab.params, source) {
                continue;
            }

            // ── checkRequirement: CheckSVar$ / SVarCompare$ ──────────
            if !valid_filter::check_svar_condition(game, &st_ab.params, source) {
                continue;
            }

            // ── checkRequirement: OnlyFirstSpell$ ────────────────────
            if st_ab
                .params
                .get("OnlyFirstSpell")
                .map(|v| v.eq_ignore_ascii_case("True"))
                .unwrap_or(false)
            {
                // Only applies if no matching spells have been cast yet this turn
                if game.player(caster).spells_cast_this_turn > 0 {
                    continue;
                }
            }

            // ── checkRequirement: Condition$ ─────────────────────────
            if !valid_filter::check_condition(game, &st_ab.params, source) {
                continue;
            }

            // ── checkRequirement: ValidTarget$ ───────────────────────
            if let Some(valid_target) = st_ab.params.get(keys::VALID_TARGET) {
                let target_valid = if targets.is_empty() {
                    // Playability: check if any valid target exists
                    game.cards.iter().any(|c| {
                        c.zone == ZoneType::Battlefield
                            && matches_valid_card(valid_target, c, source)
                    })
                } else {
                    targets.iter().any(|&tid| {
                        let target = game.card(tid);
                        matches_valid_card(valid_target, target, source)
                    })
                };
                let unless = st_ab
                    .params
                    .get("UnlessValidTarget")
                    .map(|v| v.eq_ignore_ascii_case("True"))
                    .unwrap_or(false);
                if unless {
                    if target_valid {
                        continue;
                    }
                } else if !target_valid {
                    continue;
                }
            }

            // ── checkRequirement: ValidSpell$ ────────────────────────
            if let Some(valid_spell) = st_ab.params.get(keys::VALID_SPELL) {
                if !check_valid_spell(valid_spell, spell_card) {
                    continue;
                }
            }

            // ── applyReduceCostAbility / increase: ForEachShard$ ─────
            if let Some(shard_color) = st_ab.params.get(keys::FOR_EACH_SHARD) {
                let atom =
                    forge_foundation::mana::ManaAtom::from_name(&shard_color.to_ascii_lowercase());
                let count = spell_card
                    .mana_cost
                    .shards()
                    .iter()
                    .filter(|s| (s.shard() & atom) != 0)
                    .count() as i32;
                if count == 0 {
                    continue;
                }
                if is_reduce {
                    adj.generic -= count;
                } else {
                    adj.generic += count;
                }
                continue;
            }

            // ── applyReduceCostAbility: Amount$ (with Relative$ SVar resolution) ──
            let amount: i32 = if st_ab
                .params
                .get("Relative")
                .map(|v| v.eq_ignore_ascii_case("True"))
                .unwrap_or(false)
            {
                // Relative: Amount$ is an SVar name on the source card
                let amount_str = st_ab
                    .params
                    .get(keys::AMOUNT)
                    .unwrap_or("1");
                resolve_svar_for_cost(game, source, amount_str, caster)
            } else {
                st_ab
                    .params
                    .get(keys::AMOUNT)
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(1)
            };

            // ── applyReduceCostAbility: MinMana$ ─────────────────────
            if let Some(min_str) = st_ab.params.get(keys::MIN_MANA) {
                if let Ok(min_val) = min_str.parse::<i32>() {
                    adj.min_mana = Some(match adj.min_mana {
                        Some(existing) => existing.max(min_val),
                        None => min_val,
                    });
                }
            }

            // ── applySetCostAbility: SetCost + RaiseTo$ (Trinisphere) ──
            if is_set_cost {
                if st_ab
                    .params
                    .get("RaiseTo")
                    .map(|v| v.eq_ignore_ascii_case("True"))
                    .unwrap_or(false)
                {
                    adj.raise_to = Some(match adj.raise_to {
                        Some(existing) => existing.max(amount),
                        None => amount,
                    });
                }
                continue;
            }

            // ── applyReduceCostAbility: Color$ parameter ─────────────
            if let Some(color_str) = st_ab.params.get(keys::COLOR) {
                let ignore_generic = st_ab
                    .params
                    .get("IgnoreGeneric")
                    .map(|v| v.eq_ignore_ascii_case("True"))
                    .unwrap_or(false);

                for token in color_str.split_whitespace() {
                    if let Some(color) = Color::from_name(token) {
                        if is_reduce {
                            adj.color_reductions.push((color, amount, ignore_generic));
                        } else {
                            adj.color_increases.push((color, amount));
                        }
                    }
                }
                continue; // Color$ is exclusive — don't also adjust generic
            }

            // ── Generic adjustment ───────────────────────────────────
            if is_reduce {
                adj.generic -= amount;
            } else {
                adj.generic += amount;
            }
        }
    }

    adj
}

// ── Public API: compute_raise_cost_parts ─────────────────────────────

/// Compute additional non-standard cost parts contributed by `Mode$ RaiseCost`
/// static abilities (Java: `CostAdjustment.applyRaiseCostAbility` with `Cost$...`).
///
/// Returned `Cost` can contain mana and non-mana parts. Callers should:
/// - include mana parts in spell mana affordability/payment, and
/// - route non-mana parts through normal additional-cost payment plumbing.
pub fn compute_raise_cost_parts(
    game: &GameState,
    spell_card: &Card,
    caster: PlayerId,
    cast_zone: ZoneType,
) -> Option<Cost> {
    compute_raise_cost_parts_with_targets(game, spell_card, caster, cast_zone, &[])
}

/// Like `compute_raise_cost_parts`, but checks `ValidTarget$` against chosen targets.
pub fn compute_raise_cost_parts_with_targets(
    game: &GameState,
    spell_card: &Card,
    caster: PlayerId,
    cast_zone: ZoneType,
    targets: &[CardId],
) -> Option<Cost> {
    let mut merged_parts = Vec::new();
    let mut has_tap = false;
    let mut mandatory = false;

    for source in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield || c.id == spell_card.id)
    {
        for st_ab in source.static_abilities.iter() {
            if st_ab.mode != StaticMode::IncreaseCost {
                continue;
            }

            let Some(scost) = st_ab.params.get(keys::COST) else {
                continue;
            };

            // ── checkRequirement ─────────────────────────────────────
            if let Some(type_filter) = st_ab.params.get(keys::TYPE) {
                match type_filter.to_ascii_lowercase().as_str() {
                    "spell" => {}
                    _ => continue,
                }
            }

            if let Some(activator) = st_ab.params.get(keys::ACTIVATOR) {
                match activator.to_ascii_lowercase().as_str() {
                    "you" => {
                        if source.controller != caster {
                            continue;
                        }
                    }
                    "opponent" => {
                        if source.controller == caster {
                            continue;
                        }
                    }
                    _ => {
                        eprintln!("[WARN] Unknown IncreaseCost Activator: {:?}", activator);
                    }
                }
            } else {
                // IncreaseCost without Activator$ → universal effect (e.g. Thalia)
            }

            if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
                if !matches_valid_card(valid_card, spell_card, source) {
                    continue;
                }
            }

            if let Some(zone_str) = st_ab
                .params
                .get("EffectZone")
                .or_else(|| st_ab.params.get(keys::AFFECTED_ZONE))
            {
                if !zone_str.eq_ignore_ascii_case("All") {
                    let zones: Vec<&str> = zone_str.split(',').map(|s| s.trim()).collect();
                    if !zones.iter().any(|z| zone_name_matches(cast_zone, z)) {
                        continue;
                    }
                }
            }

            if !valid_filter::check_is_present(game, &st_ab.params, source) {
                continue;
            }

            if !valid_filter::check_svar_condition(game, &st_ab.params, source) {
                continue;
            }

            if st_ab
                .params
                .get("OnlyFirstSpell")
                .map(|v| v.eq_ignore_ascii_case("True"))
                .unwrap_or(false)
                && game.player(caster).spells_cast_this_turn > 0
            {
                continue;
            }

            if !valid_filter::check_condition(game, &st_ab.params, source) {
                continue;
            }

            if let Some(valid_target) = st_ab.params.get(keys::VALID_TARGET) {
                let target_valid = if targets.is_empty() {
                    game.cards.iter().any(|c| {
                        c.zone == ZoneType::Battlefield
                            && matches_valid_card(valid_target, c, source)
                    })
                } else {
                    targets.iter().any(|&tid| {
                        let target = game.card(tid);
                        matches_valid_card(valid_target, target, source)
                    })
                };
                let unless = st_ab
                    .params
                    .get("UnlessValidTarget")
                    .map(|v| v.eq_ignore_ascii_case("True"))
                    .unwrap_or(false);
                if (unless && target_valid) || (!unless && !target_valid) {
                    continue;
                }
            }

            if let Some(valid_spell) = st_ab.params.get(keys::VALID_SPELL) {
                if !check_valid_spell(valid_spell, spell_card) {
                    continue;
                }
            }

            // ── applyRaiseCostAbility: compute count ─────────────────
            let count: i32 = if let Some(shard_color) = st_ab.params.get(keys::FOR_EACH_SHARD) {
                let atom =
                    forge_foundation::mana::ManaAtom::from_name(&shard_color.to_ascii_lowercase());
                spell_card
                    .mana_cost
                    .shards()
                    .iter()
                    .filter(|s| (s.shard() & atom) != 0)
                    .count() as i32
            } else if st_ab
                .params
                .get("Relative")
                .map(|v| v.eq_ignore_ascii_case("True"))
                .unwrap_or(false)
            {
                let amount_str = st_ab
                    .params
                    .get(keys::AMOUNT)
                    .unwrap_or("1");
                resolve_svar_for_cost(game, source, amount_str, caster)
            } else {
                st_ab
                    .params
                    .get(keys::AMOUNT)
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(1)
            };

            if count <= 0 {
                continue;
            }

            let parsed = parse_cost(scost);
            for _ in 0..count {
                merged_parts.extend(parsed.parts.clone());
            }
            has_tap |= parsed.has_tap;
            mandatory |= parsed.mandatory;
        }
    }

    if merged_parts.is_empty() {
        None
    } else {
        Some(Cost {
            parts: merged_parts,
            has_tap,
            mandatory,
        })
    }
}

// ── checkRequirement helpers (mirrors Java CostAdjustment.checkRequirement) ──

/// Check a ValidSpell$ parameter.
/// Mirrors the `ValidSpell` check in Java's `CostAdjustment.checkRequirement`.
fn check_valid_spell(valid_spell: &str, spell_card: &Card) -> bool {
    // Split comma-separated options — any match passes
    valid_spell.split(',').any(|option| {
        let parts: Vec<&str> = option.trim().split('.').collect();
        let category = parts.first().copied().unwrap_or("");
        match category {
            "Spell" => {
                // We're casting a spell, check sub-attributes
                parts
                    .iter()
                    .skip(1)
                    .all(|attr| match attr.to_lowercase().as_str() {
                        "bargain" => spell_card.has_keyword("Bargain"),
                        _ => true, // unknown attributes pass
                    })
            }
            "Activated" | "Static" => {
                // These are for ability cost changes, not spell casting
                false
            }
            _ => true,
        }
    })
}

// ── SVar resolution for cost adjustment context ──────────────────────

/// Resolve an SVar expression in the context of cost adjustment.
///
/// Supports:
/// - Direct SVar names on `source.svars` that contain `Count$...` expressions
/// - Numeric literals
fn resolve_svar_for_cost(
    game: &GameState,
    source: &Card,
    name: &str,
    caster: PlayerId,
) -> i32 {
    // If it's a direct number, return it
    if let Ok(n) = name.parse::<i32>() {
        return n;
    }

    // Look up in source card's SVars
    let expr = match source.svars.get(name) {
        Some(e) => e.as_str(),
        None => return 0,
    };

    evaluate_count_expr(game, source, expr, caster)
}

/// Evaluate a `Count$...` expression.
fn evaluate_count_expr(
    game: &GameState,
    source: &Card,
    expr: &str,
    caster: PlayerId,
) -> i32 {
    // Count$ThisTurnCast_Card.YouCtrl — spells cast this turn by controller
    if let Some(rest) = expr.strip_prefix("Count$ThisTurnCast_") {
        if rest.contains("YouCtrl") || rest.contains("YouOwn") {
            return game.player(source.controller).spells_cast_this_turn;
        }
        // Generic: all players' spells this turn (approximate)
        return game.player(caster).spells_cast_this_turn;
    }

    // Count$YourLifeTotal
    if expr == "Count$YourLifeTotal" {
        return game.player(source.controller).life;
    }

    // Count$CardsInYourGraveyard or Count$TypeYouCtrl.Graveyard
    if expr.contains("Graveyard") && expr.contains("YouCtrl") {
        return game
            .cards_in_zone(ZoneType::Graveyard, source.controller)
            .len() as i32;
    }

    // Count$CreatureYouCtrl
    if expr.contains("Creature") && expr.contains("YouCtrl") {
        return game
            .cards_in_zone(ZoneType::Battlefield, source.controller)
            .iter()
            .filter(|&&cid| game.card(cid).is_creature())
            .count() as i32;
    }

    // Fallback: try numeric
    expr.strip_prefix("Count$")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}


// ── Helper: zone name matching ────────────────────────────────────────

fn zone_name_matches(zone: ZoneType, name: &str) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "hand" => zone == ZoneType::Hand,
        "command" => zone == ZoneType::Command,
        "graveyard" => zone == ZoneType::Graveyard,
        "exile" => zone == ZoneType::Exile,
        "battlefield" => zone == ZoneType::Battlefield,
        "library" => zone == ZoneType::Library,
        "stack" => zone == ZoneType::Stack,
        "all" => true,
        _ => true,
    }
}

// ── ValidCard$ matching (mirrors Java's checkRequirement ValidCard) ──

pub(crate) fn matches_valid_card(valid: &str, spell: &Card, source: &Card) -> bool {
    valid_filter::matches_valid_card(valid, spell, source)
}

// ── Affinity / Delve / Convoke / Improvise helpers ──────────────────
// These were in game_action_util.rs but belong in the cost module,
// mirroring Java's CostAdjustment which handles convoke/improvise/delve
// via adjustCostByConvokeOrImprovise() and adjustCostByOffering() etc.

/// Extract the affinity type from a card's keywords (e.g. "Affinity:Artifact" → "Artifact").
/// Mirrors Java's affinity keyword handling in `CostAdjustment`.
pub fn get_affinity_type(card: &Card) -> Option<String> {
    crate::keyword::extract_keyword_cost_from_all(
        [&card.keywords, &card.granted_keywords],
        "Affinity",
    )
}

/// Count permanents matching an affinity type on the battlefield.
/// Mirrors Java's affinity permanent counting in `CostAdjustment`.
pub fn count_affinity_permanents(
    game: &GameState,
    player: PlayerId,
    affinity_type: &str,
    exclude_card: CardId,
) -> i32 {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .iter()
        .filter(|&&cid| {
            if cid == exclude_card {
                return false;
            }
            let c = game.card(cid);
            match affinity_type {
                "Artifact" => c.type_line.is_artifact(),
                "Creature" => c.is_creature(),
                "Enchantment" => c.type_line.is_enchantment(),
                "Land" => c.is_land(),
                "Planeswalker" => c.type_line.is_planeswalker(),
                other => c.type_line.has_subtype(other),
            }
        })
        .count() as i32
}

/// Apply Delve/Convoke/Improvise/Affinity generic cost reductions.
/// Used for `canPay` checks — estimates the maximum possible reduction.
/// Mirrors Java's CostAdjustment adjustment methods for these mechanics.
pub fn apply_cost_reductions(
    game: &GameState,
    player: PlayerId,
    card_id: CardId,
    card: &Card,
    cost: &ManaCost,
) -> ManaCost {
    if card.has_keyword("Delve") {
        let gy_count = game
            .cards_in_zone(ZoneType::Graveyard, player)
            .iter()
            .filter(|&&cid| cid != card_id)
            .count() as i32;
        cost.reduce_generic(gy_count)
    } else if card.has_keyword("Convoke") {
        let creature_count = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .filter(|&&cid| {
                let c = game.card(cid);
                c.is_creature() && !c.tapped && cid != card_id
            })
            .count() as i32;
        cost.reduce_generic(creature_count)
    } else if card.has_keyword("Improvise") {
        let artifact_count = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .filter(|&&cid| {
                let c = game.card(cid);
                c.type_line.is_artifact() && !c.tapped && cid != card_id
            })
            .count() as i32;
        cost.reduce_generic(artifact_count)
    } else if let Some(affinity_type) = get_affinity_type(card) {
        let count = count_affinity_permanents(game, player, &affinity_type, card_id);
        cost.reduce_generic(count)
    } else {
        cost.clone()
    }
}
