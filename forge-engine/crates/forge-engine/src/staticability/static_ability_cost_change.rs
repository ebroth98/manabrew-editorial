use forge_foundation::color::Color;
use forge_foundation::mana::ManaCost;
use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

/// Result of computing cost adjustments from static abilities.
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

/// Compute cost adjustments for casting `spell_card` by `caster` from `cast_zone`.
///
/// Scans all battlefield permanents for ReduceCost / IncreaseCost / SetCost static abilities.
pub fn compute_cost_adjustment(
    game: &GameState,
    spell_card: &CardInstance,
    caster: PlayerId,
    cast_zone: ZoneType,
) -> CostAdjustment {
    let mut adj = CostAdjustment::default();

    for source in game.cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
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

            // ── Type$ filter ──────────────────────────────────────────
            if let Some(type_filter) = st_ab.params.get("Type") {
                match type_filter.to_ascii_lowercase().as_str() {
                    "spell" => { /* casting a spell — ok */ }
                    _ => continue,
                }
            }

            // ── Activator$ ────────────────────────────────────────────
            if let Some(activator) = st_ab.params.get("Activator") {
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
                    _ => {} // "Player" or unknown → applies to all
                }
            } else {
                // Default: applies to controller only
                if source.controller != caster {
                    continue;
                }
            }

            // ── ValidCard$ ────────────────────────────────────────────
            if let Some(valid_card) = st_ab.params.get("ValidCard") {
                if !matches_valid_card(valid_card, spell_card, source) {
                    continue;
                }
            }

            // ── EffectZone$ / AffectedZone$ ───────────────────────────
            if let Some(zone_str) = st_ab
                .params
                .get("EffectZone")
                .or_else(|| st_ab.params.get("AffectedZone"))
            {
                if !zone_str.eq_ignore_ascii_case("All") {
                    let zones: Vec<&str> = zone_str.split(',').map(|s| s.trim()).collect();
                    if !zones.iter().any(|z| zone_name_matches(cast_zone, z)) {
                        continue;
                    }
                }
            }

            // ── IsPresent$ / PresentZone$ condition ───────────────────
            if let Some(condition) = st_ab.params.get("IsPresent") {
                let present_zone = st_ab
                    .params
                    .get("PresentZone")
                    .map(String::as_str)
                    .unwrap_or("Battlefield");
                if !check_is_present(game, caster, condition, present_zone) {
                    continue;
                }
            }

            // ── CheckSVar$ / SVarCompare$ condition ───────────────────
            if let Some(check_name) = st_ab.params.get("CheckSVar") {
                if let Some(compare) = st_ab.params.get("SVarCompare") {
                    let value = resolve_svar_for_cost(game, source, check_name, caster);
                    if !compare_svar(value, compare) {
                        continue;
                    }
                }
            }

            // ── OnlyFirstSpell$ ───────────────────────────────────────
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

            // ── ValidTarget$ ──────────────────────────────────────────
            // Target-dependent cost reduction: at playability-check time we don't
            // know the targets yet, so we conservatively skip this ability.
            // The reduction will still appear once the spell is on the stack with
            // targets chosen (future enhancement).
            if st_ab.params.contains_key("ValidTarget") {
                continue;
            }

            // ── ValidSpell$ ───────────────────────────────────────────
            // Spell-attribute filtering (e.g. Bargain, Kicked, MayPlaySource).
            // At cost-check time we don't know these attributes, so skip.
            if st_ab.params.contains_key("ValidSpell") {
                continue;
            }

            // ── Amount$ (with Relative$ SVar resolution) ──────────────
            let amount: i32 = if st_ab
                .params
                .get("Relative")
                .map(|v| v.eq_ignore_ascii_case("True"))
                .unwrap_or(false)
            {
                // Relative: Amount$ is an SVar name on the source card
                let amount_str = st_ab.params.get("Amount").map(String::as_str).unwrap_or("1");
                resolve_svar_for_cost(game, source, amount_str, caster)
            } else {
                st_ab
                    .params
                    .get("Amount")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(1)
            };

            // ── UpTo$ — reduction is optional; AI always applies full amount ──
            // (No change needed — we always apply the full reduction.
            //  A future UI enhancement could let human players choose.)

            // ── MinMana$ ─────────────────────────────────────────────
            if let Some(min_str) = st_ab.params.get("MinMana") {
                if let Ok(min_val) = min_str.parse::<i32>() {
                    adj.min_mana = Some(match adj.min_mana {
                        Some(existing) => existing.max(min_val),
                        None => min_val,
                    });
                }
            }

            // ── SetCost + RaiseTo$ (Trinisphere) ─────────────────────
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

            // ── Color$ parameter ─────────────────────────────────────
            if let Some(color_str) = st_ab.params.get("Color") {
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

// ── SVar resolution for cost adjustment context ──────────────────────

/// Resolve an SVar expression in the context of cost adjustment.
///
/// Supports:
/// - Direct SVar names on `source.svars` that contain `Count$...` expressions
/// - Numeric literals
fn resolve_svar_for_cost(
    game: &GameState,
    source: &CardInstance,
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
    source: &CardInstance,
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

/// Compare a value against an SVarCompare$ string (e.g. "GE1", "EQ0", "LE3").
fn compare_svar(value: i32, compare: &str) -> bool {
    if let Some(n) = compare.strip_prefix("GE").and_then(|s| s.parse::<i32>().ok()) {
        return value >= n;
    }
    if let Some(n) = compare.strip_prefix("GT").and_then(|s| s.parse::<i32>().ok()) {
        return value > n;
    }
    if let Some(n) = compare.strip_prefix("LE").and_then(|s| s.parse::<i32>().ok()) {
        return value <= n;
    }
    if let Some(n) = compare.strip_prefix("LT").and_then(|s| s.parse::<i32>().ok()) {
        return value < n;
    }
    if let Some(n) = compare.strip_prefix("EQ").and_then(|s| s.parse::<i32>().ok()) {
        return value == n;
    }
    if let Some(n) = compare.strip_prefix("NE").and_then(|s| s.parse::<i32>().ok()) {
        return value != n;
    }
    true // unknown comparator → pass
}

// ── Helper: check IsPresent$ condition ────────────────────────────────

fn check_is_present(
    game: &GameState,
    player: PlayerId,
    condition: &str,
    present_zone: &str,
) -> bool {
    let zone = match present_zone.to_ascii_lowercase().as_str() {
        "battlefield" => ZoneType::Battlefield,
        "graveyard" => ZoneType::Graveyard,
        "exile" => ZoneType::Exile,
        "hand" => ZoneType::Hand,
        "library" => ZoneType::Library,
        _ => ZoneType::Battlefield,
    };

    let (type_part, qualifier) = if let Some((t, q)) = condition.split_once('.') {
        (t, Some(q))
    } else {
        (condition, None)
    };

    let is_you_ctrl = qualifier
        .map(|q| q.eq_ignore_ascii_case("YouCtrl") || q.eq_ignore_ascii_case("YouOwn"))
        .unwrap_or(true);

    let cards = if is_you_ctrl {
        game.cards_in_zone(zone, player).to_vec()
    } else {
        let opp = game.opponent_of(player);
        game.cards_in_zone(zone, opp).to_vec()
    };

    cards.iter().any(|&cid| {
        let card = game.card(cid);
        card.type_line.has_subtype(type_part)
            || matches_base_type(&type_part.to_ascii_lowercase(), card)
    })
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

// ── ValidCard$ matching ──────────────────────────────────────────────

fn matches_valid_card(valid: &str, spell: &CardInstance, source: &CardInstance) -> bool {
    let alternatives: Vec<&str> = valid.split(',').map(|s| s.trim()).collect();
    alternatives
        .iter()
        .any(|alt| matches_single_valid(alt, spell, source))
}

fn matches_single_valid(token: &str, spell: &CardInstance, source: &CardInstance) -> bool {
    if let Some((base, qualifier)) = token.split_once('.') {
        let base_lower = base.to_ascii_lowercase();
        let qual_lower = qualifier.to_ascii_lowercase();

        match qual_lower.as_str() {
            "self" => return spell.id == source.id,
            "noncreature" => return !spell.is_creature(),
            "nonland" => return !spell.is_land(),
            "multicolor" => return spell.color.is_multicolor(),
            _ => {}
        }

        // Color qualifiers on Card base
        if base_lower == "card" {
            if let Some(color) = Color::from_name(&qual_lower) {
                return spell.color.has_color(color);
            }
            if qual_lower == "colorless" {
                return spell.color.is_colorless();
            }
        }

        // Base type + qualifier (fallback)
        if !matches_base_type(&base_lower, spell) {
            return false;
        }
        true
    } else {
        let lower = token.to_ascii_lowercase();
        matches_base_type(&lower, spell)
    }
}

fn matches_base_type(base: &str, spell: &CardInstance) -> bool {
    match base {
        "card" => true,
        "creature" => spell.is_creature(),
        "instant" => spell.type_line.is_instant(),
        "sorcery" => spell.type_line.is_sorcery(),
        "artifact" => spell.type_line.is_artifact(),
        "enchantment" => spell.type_line.is_enchantment(),
        "planeswalker" => spell.type_line.is_planeswalker(),
        "land" => spell.is_land(),
        _ => {
            // Try matching as subtype (e.g. "Wizard", "Dragon")
            spell.type_line.has_subtype(base)
        }
    }
}
