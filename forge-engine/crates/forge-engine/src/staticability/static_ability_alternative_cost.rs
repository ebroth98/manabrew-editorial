use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::cost::{parse_cost, Cost};
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

/// Descriptor for an alternative cost produced by a static ability.
/// Mirrors the `SpellAbility` objects returned by Java's
/// `StaticAbilityAlternativeCost.alternativeCosts()`.
///
/// Because the Rust `SpellAbility` doesn't yet support `copyWithDefinedCost` /
/// `copyWithManaCostReplaced`, we capture the validated data so that downstream
/// code can construct the actual SA when ready.
#[derive(Debug, Clone)]
pub struct AlternativeCostEntry {
    /// The parsed alternative cost (Cost$ param with ConvertedManaCost replaced).
    pub cost: Cost,
    /// The raw cost template string (after ConvertedManaCost substitution).
    pub cost_string: String,
    /// Whether the original SA is an ability (not a spell).
    pub is_ability: bool,
    /// Whether the original SA is a spell.
    pub is_spell: bool,
    /// Propagated XAlternative param, if any.
    pub x_alternative: Option<String>,
    /// Propagated Announce param, if any.
    pub announce: Option<String>,
    /// Propagated ManaRestriction param, if any.
    pub mana_restriction: Option<String>,
    /// Zone restriction from the static ability's ActiveZones (single zone case).
    pub zone_restriction: Option<ZoneType>,
    /// StackDescription override, if any.
    pub stack_description: Option<String>,
    /// CostDesc override, if any.
    pub cost_desc: Option<String>,
    /// Description text for the alternative cost.
    pub description: String,
    /// Named override — custom card alternative cost naming.
    pub named: Option<String>,
    /// Whether this is an AltCost from the source card itself (adds OptionalCost.AltCost).
    pub is_own_alt_cost: bool,
}

/// Collect all alternative costs that apply to `sa` cast by `player` from `source`.
/// Mirrors Java's `StaticAbilityAlternativeCost.alternativeCosts()`.
///
/// Iterates over `source` itself (in case it's LKI / alternate host) plus all
/// cards in static-ability source zones, checking each `AlternativeCost` static
/// ability for applicability.
pub fn alternative_costs(
    game: &GameState,
    cards: &[Card],
    sa: &SpellAbility,
    source: &Card,
    player: PlayerId,
) -> Vec<AlternativeCostEntry> {
    let mut result = Vec::new();

    // Java: add source first in case it's LKI (alternate host),
    // then all cards from STATIC_ABILITIES_SOURCE_ZONES.
    // We iterate source first, then all cards in static ability source zones,
    // skipping source if it appears again.
    let source_id = source.id;

    // Process source card first
    collect_from_card(game, source, sa, source, player, &mut result);

    // Process all cards in static ability source zones
    for ca in cards
        .iter()
        .filter(|c| c.zone.is_static_ability_source() && c.id != source_id)
    {
        collect_from_card(game, ca, sa, source, player, &mut result);
    }

    result
}

/// Check if any alternative cost static applies to this card for this player.
/// Convenience wrapper around `alternative_costs` — returns true if at least one
/// alternative cost is available.
pub fn has_alternative_cost(
    game: &GameState,
    cards: &[Card],
    sa: &SpellAbility,
    source: &Card,
    player: PlayerId,
) -> bool {
    let source_id = source.id;

    // Check source card first
    for st_ab in source
        .static_abilities
        .iter()
        .filter(|s| s.is_active_for(StaticMode::AlternativeCost, source.zone))
    {
        if !st_ab.check_conditions(source, game) {
            continue;
        }
        if apply(st_ab, sa, source, source, player) {
            return true;
        }
    }

    // Check all cards in static ability source zones
    for ca in cards
        .iter()
        .filter(|c| c.zone.is_static_ability_source() && c.id != source_id)
    {
        for st_ab in ca
            .static_abilities
            .iter()
            .filter(|s| s.is_active_for(StaticMode::AlternativeCost, ca.zone))
        {
            if !st_ab.check_conditions(ca, game) {
                continue;
            }
            if apply(st_ab, sa, source, ca, player) {
                return true;
            }
        }
    }

    false
}

/// Process all static abilities on a single card, collecting matching
/// alternative cost entries.
fn collect_from_card(
    game: &GameState,
    ca: &Card,
    sa: &SpellAbility,
    source: &Card,
    player: PlayerId,
    result: &mut Vec<AlternativeCostEntry>,
) {
    for st_ab in ca
        .static_abilities
        .iter()
        .filter(|s| s.is_active_for(StaticMode::AlternativeCost, ca.zone))
    {
        if !st_ab.check_conditions(ca, game) {
            continue;
        }
        if !apply(st_ab, sa, source, ca, player) {
            continue;
        }

        // Parse and substitute Cost$ param
        let Some(cost_template_raw) = st_ab.params.get(keys::COST) else {
            continue;
        };

        // Replace "ConvertedManaCost" with the source card's CMC
        let cmc = source.mana_value();
        let cost_template = cost_template_raw.replace("ConvertedManaCost", &cmc.to_string());

        let is_ability = sa.is_activated;
        let is_spell = sa.is_spell;

        let cost = parse_cost(&cost_template);

        // Propagate XAlternative param
        let x_alternative = st_ab.params.get("XAlternative").map(|s| s.to_string());

        // Propagate Announce param
        let announce = st_ab.params.get("Announce").map(|s| s.to_string());

        // Propagate ManaRestriction param
        let mana_restriction = st_ab.params.get("ManaRestriction").map(|s| s.to_string());

        // Zone restriction from ActiveZones — only if host card is not immutable
        // (Rust Card has no is_immutable field yet, so we skip that guard)
        // TODO: Add is_immutable check when Card gains that field
        let zone_restriction = {
            if let Some(zones_str) = st_ab.params.get(keys::ACTIVE_ZONES) {
                let zones: Vec<&str> = zones_str.split(',').map(str::trim).collect();
                if zones.len() == 1 {
                    parse_zone_type(zones[0])
                } else {
                    None
                }
            } else {
                None
            }
        };

        // StackDescription override
        let stack_description = st_ab.params.get("StackDescription").map(|s| s.to_string());

        // Build description — mirrors Java's SpellDescription construction
        let mut desc = String::new();
        let cost_desc = if is_ability {
            // CostDesc for abilities
            let cd = if let Some(cd_param) = st_ab.params.get("CostDesc") {
                // Java: ManaCostParser.parse(stAb.getParam("CostDesc"))
                // For now, pass through as-is
                // TODO: implement ManaCostParser.parse() equivalent
                cd_param.to_string()
            } else {
                cost_template.clone()
            };
            desc.push_str(&cd);
            Some(cd)
        } else {
            None
        };

        if is_spell {
            // Append the original spell description
            // TODO: sa.getDescription() equivalent — use ability_text as fallback
            desc.push_str(&sa.ability_text);

            // Check if source card is the host card of the static ability
            if source.id == ca.id {
                // Same card — use the Description param
                if let Some(alt_desc) = st_ab.params.get(keys::DESCRIPTION) {
                    desc.push_str(" (");
                    desc.push_str(alt_desc);
                    desc.push_str(") ");
                }
            } else {
                // Different card — generic "by paying X instead of its mana cost"
                desc.push_str(" (by paying ");
                desc.push_str(&cost_template);
                desc.push_str(" instead of its mana cost)");
            }
        }

        // Whether this is the card's own alt cost (same host card)
        let is_own_alt_cost = is_spell && source.id == ca.id;

        // Named param override for custom cards
        let named = st_ab.params.get("Named").map(|s| s.to_string());

        result.push(AlternativeCostEntry {
            cost,
            cost_string: cost_template,
            is_ability,
            is_spell,
            x_alternative,
            announce,
            mana_restriction,
            zone_restriction,
            stack_description,
            cost_desc,
            description: desc,
            named,
            is_own_alt_cost,
        });
    }
}

/// Validation logic — mirrors Java's `StaticAbilityAlternativeCost.apply()`.
///
/// Checks ValidSA, ValidCard, and ValidPlayer params against the spell ability,
/// source card, and player.
fn apply(
    st_ab: &crate::staticability::StaticAbility,
    sa: &SpellAbility,
    source: &Card,
    host: &Card,
    player: PlayerId,
) -> bool {
    // Check ValidSA — mirrors Java's stAb.matchesValidParam("ValidSA", sa)
    if let Some(valid_sa) = st_ab.params.get(keys::VALID_SA) {
        if !spell_ability_matches(valid_sa, sa, source, host) {
            return false;
        }
    }

    // Check ValidCard — mirrors Java's stAb.matchesValidParam("ValidCard", source)
    if !valid_filter::matches_valid_card_opt(st_ab.params.get(keys::VALID_CARD), source, host) {
        return false;
    }

    // Check ValidPlayer — mirrors Java's stAb.matchesValidParam("ValidPlayer", pl)
    if !valid_filter::matches_valid_player_opt(
        st_ab.params.get(keys::VALID_PLAYER),
        player,
        host.controller,
    ) {
        return false;
    }

    true
}

/// Check if a SpellAbility matches a ValidSA filter string.
/// The filter is comma-separated; all tokens must match (AND logic), mirroring
/// Java's `matchesValid` which splits on comma and requires all to pass.
///
/// Recognised tokens (case-insensitive):
/// - "Spell" — the SA must be a spell (has SP$ param or is_spell)
/// - "Ability" — the SA must be an ability
fn spell_ability_matches(valid_sa: &str, sa: &SpellAbility, source: &Card, host: &Card) -> bool {
    let tokens: Vec<&str> = valid_sa
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return true;
    }

    tokens.iter().all(|tok| {
        let parts: Vec<&str> = tok
            .split('.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let primary = parts.first().copied().unwrap_or("");
        let lower = primary.to_ascii_lowercase();

        let base_ok = match lower.as_str() {
            "spell" => sa.is_spell || sa.params.has(keys::SP),
            "ability" => sa.is_activated || sa.is_trigger,
            _ => {
                if let Some(ref api) = sa.api {
                    api.name().eq_ignore_ascii_case(primary)
                } else {
                    false
                }
            }
        };
        if !base_ok {
            return false;
        }

        if parts.len() >= 2 {
            match parts[1].to_ascii_lowercase().as_str() {
                // ValidSA$ Spell.Self should only match the source card's own spell.
                "self" => {
                    if sa.source != Some(source.id) || source.id != host.id {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    })
}

/// Parse a zone type string into a `ZoneType`.
fn parse_zone_type(s: &str) -> Option<ZoneType> {
    let lower = s.trim().to_ascii_lowercase();
    match lower.as_str() {
        "battlefield" => Some(ZoneType::Battlefield),
        "hand" => Some(ZoneType::Hand),
        "graveyard" => Some(ZoneType::Graveyard),
        "library" => Some(ZoneType::Library),
        "exile" => Some(ZoneType::Exile),
        "command" => Some(ZoneType::Command),
        "stack" => Some(ZoneType::Stack),
        _ => None,
    }
}

/// Apply an `AlternativeCostEntry` to a `SpellAbility`, mutating it in place
/// to reflect the alternative cost. This is a partial implementation of Java's
/// `sa.copyWithDefinedCost(cost)` / `sa.copyWithManaCostReplaced(pl, cost)`.
///
/// TODO: Full implementation requires SpellAbility to support:
/// - `copyWithDefinedCost(cost)` for abilities
/// - `copyWithManaCostReplaced(player, cost)` for spells
/// - `addOptionalCost(OptionalCost.AltCost)` for own alt costs
/// For now, this applies what we can: params, description, pay_costs.
pub fn apply_alternative_cost_to_sa(sa: &mut SpellAbility, entry: &AlternativeCostEntry) {
    // Replace the pay costs with the alternative cost
    sa.pay_costs = Some(entry.cost.clone());

    // Mark as not a basic spell
    // TODO: sa.setBasicSpell(false) — no field yet

    // Propagate params
    if let Some(ref x_alt) = entry.x_alternative {
        sa.params.put("XAlternative".to_string(), x_alt.clone());
    }
    if let Some(ref announce) = entry.announce {
        sa.params.put("Announce".to_string(), announce.clone());
    }
    if let Some(ref mana_restriction) = entry.mana_restriction {
        sa.params
            .put("ManaRestriction".to_string(), mana_restriction.clone());
    }
    if let Some(ref stack_desc) = entry.stack_description {
        sa.params
            .put("StackDescription".to_string(), stack_desc.clone());
    }
    if let Some(ref cost_desc) = entry.cost_desc {
        sa.params.put("CostDesc".to_string(), cost_desc.clone());
    }

    // Apply Named override
    if let Some(ref named) = entry.named {
        sa.params.put(keys::NAME.to_string(), named.clone());
    }

    // TODO: Zone restriction on sa.restrictions when SpellAbilityRestriction is ported
    // TODO: addOptionalCost(OptionalCost.AltCost) for is_own_alt_cost
}
