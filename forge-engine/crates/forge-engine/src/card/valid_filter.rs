//! Shared card/player filter matching used across triggers, static abilities,
//! replacement effects, and combat. Consolidates 34 duplicate implementations.
//!
//! This module provides canonical implementations for matching cards and players
//! against filter expressions like "Creature.YouCtrl" or "Opponent".
//!
//! **Filter Syntax:**
//!
//! Card filters are dot-separated: "Creature.YouCtrl.nonToken"
//! - Comma separates OR conditions: "Creature,Artifact" matches either
//! - Dot separates qualifiers: "Creature.YouCtrl" matches creatures you control
//! - Plus separates compound conditions: "YouCtrl+kicked" matches both
//!
//! **Type parts:**
//! - Card, Permanent, Creature, Land, Artifact, Enchantment, Planeswalker, Instant, Sorcery
//! - Subtypes: "Zombie", "Wall", "Forest", etc.
//!
//! **Qualifiers:**
//! - Controller: YouCtrl, OppCtrl, YouControl, OpponentCtrl
//! - Self: Self, Other, StrictlyOther
//! - Token: token, nonToken
//! - Type negation: nonCreature, nonLand
//! - State: tapped, untapped, kicked
//! - Counters: counters_GE3_P1P1, counters_EQ1_Charge
//! - CMC: cmcEQ1, cmcLE3, cmcGE5
//! - Color: White, Blue, Black, Red, Green, Colorless, multicolor
//! - Combat: DamagedBy
//! - Attachment: EnchantedBy

use forge_foundation::color::Color;
use forge_foundation::ZoneType;

use crate::card::Card;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;
use crate::parsing::Params;

/// Check if a card matches a filter expression like "Creature.YouCtrl".
/// Returns true if `valid` is empty or the card satisfies all parts.
///
/// # Examples
///
/// ```ignore
/// // Matches any creature you control:
/// matches_valid_card("Creature.YouCtrl", creature, source)
///
/// // Matches either creatures or artifacts:
/// matches_valid_card("Creature,Artifact", card, source)
///
/// // Matches creatures you control that are tokens:
/// matches_valid_card("Creature.YouCtrl.token", card, source)
/// ```
pub fn matches_valid_card(valid: &str, card: &Card, source: &Card) -> bool {
    let valid = valid.trim();
    if valid.is_empty() {
        return true;
    }

    // Comma-separated = OR conditions (only if no dots, to avoid splitting "Type.Qualifier,Other")
    if valid.contains(',') && !valid.contains('.') {
        return valid
            .split(',')
            .any(|part| matches_single_valid_card(part.trim(), card, source));
    }

    matches_single_valid_card(valid, card, source)
}

/// Convenience wrapper: None means "no filter" → always matches.
pub fn matches_valid_card_opt(valid: Option<&str>, card: &Card, source: &Card) -> bool {
    match valid {
        None => true,
        Some(v) => matches_valid_card(v, card, source),
    }
}

fn matches_single_valid_card(filter: &str, card: &Card, source: &Card) -> bool {
    // Handle comma-separated types with qualifiers (e.g. "Creature.YouCtrl,Artifact.YouCtrl")
    if filter.contains(',') {
        return filter
            .split(',')
            .any(|alt| matches_type_and_qualifiers(alt.trim(), card, source));
    }

    matches_type_and_qualifiers(filter, card, source)
}

fn matches_type_and_qualifiers(filter: &str, card: &Card, source: &Card) -> bool {
    // Split on dots for compound filters (e.g. "Creature.Other", "Card.Self")
    let parts: Vec<&str> = filter.split('.').collect();
    if parts.is_empty() {
        return true;
    }

    let type_part = parts[0];
    let qualifiers = &parts[1..];

    // Check the type portion
    let type_matches = match type_part {
        "Card" | "Any" => true, // matches any card
        "Creature" => card.is_creature(),
        "Land" => card.is_land(),
        "Instant" => card.type_line.is_instant(),
        "Sorcery" => card.type_line.is_sorcery(),
        "Artifact" => card.type_line.is_artifact(),
        "Enchantment" => card.type_line.is_enchantment(),
        "Planeswalker" => card.type_line.is_planeswalker(),
        "Permanent" => card.is_permanent(),
        "Spell" => true, // used in some contexts
        // Player-type filters: players are not cards, so never match.
        "Player" | "You" | "Opponent" | "Each" | "ActivePlayer" | "NonActivePlayer" => false,
        _ => {
            // Try comma-separated types within the type portion (e.g. "Instant,Sorcery")
            if type_part.contains(',') {
                type_part.split(',').any(|t| match t.trim() {
                    "Creature" => card.is_creature(),
                    "Land" => card.is_land(),
                    "Instant" => card.type_line.is_instant(),
                    "Sorcery" => card.type_line.is_sorcery(),
                    "Artifact" => card.type_line.is_artifact(),
                    "Enchantment" => card.type_line.is_enchantment(),
                    "Planeswalker" => card.type_line.is_planeswalker(),
                    "Card" => true,
                    _ => false,
                })
            } else {
                // Try matching as subtype (e.g. "Zombie", "Wall", "Dragon").
                // This must be changeling-aware for creature types, matching
                // Java's CardType.hasStringType()/hasCreatureType() path.
                card.has_subtype(type_part)
            }
        }
    };

    if !type_matches {
        return false;
    }

    // Check qualifiers — handle compound "+" syntax (e.g. "Self+kicked", "YouCtrl+nonBlack")
    for &qualifier in qualifiers {
        // Split compound qualifiers on '+' (e.g. "Self+kicked" → ["Self", "kicked"])
        let sub_parts: Vec<&str> = qualifier.split('+').collect();
        for sub in &sub_parts {
            // Handle "!" prefix as negation (e.g. "!token" → "nontoken")
            let (negated, raw) = if let Some(stripped) = sub.strip_prefix('!') {
                (true, stripped)
            } else {
                (false, *sub)
            };
            let sub_lower = raw.to_ascii_lowercase();
            // If negated, invert the boolean result of the positive match.
            // "!token" is equivalent to "nontoken", "!Creature" to "nonCreature", etc.
            if negated {
                let positive_match = match sub_lower.as_str() {
                    "token" => card.is_token,
                    "creature" => card.is_creature(),
                    "land" => card.is_land(),
                    "artifact" => card.type_line.is_artifact(),
                    "enchantment" => card.type_line.is_enchantment(),
                    "legendary" => card.type_line.is_legendary(),
                    _ => {
                        // Try subtype match
                        card.has_subtype(raw)
                    }
                };
                if positive_match {
                    return false;
                }
                continue;
            }
            match sub_lower.as_str() {
                "self" => {
                    if card.id != source.id {
                        return false;
                    }
                }
                "other" | "strictlyother" => {
                    if card.id == source.id {
                        return false;
                    }
                }
                "youctrl" | "youcontrol" | "you" => {
                    if card.controller != source.controller {
                        return false;
                    }
                }
                "youown" => {
                    if card.owner != source.controller {
                        return false;
                    }
                }
                "isremembered" => {
                    if !source.remembered_cards.contains(&card.id) {
                        return false;
                    }
                }
                "effectsource" => {
                    if source.effect_source != Some(card.id) {
                        return false;
                    }
                }
                "oppctrl" | "opponentctrl" | "opponent" => {
                    if card.controller == source.controller {
                        return false;
                    }
                }
                "oppown" | "opponentown" => {
                    if card.owner == source.controller {
                        return false;
                    }
                }
                "iscommander" => {
                    if !card.is_commander {
                        return false;
                    }
                }
                "kicked" => {
                    if !card.kicked {
                        return false;
                    }
                }
                "noncreature" => {
                    if card.is_creature() {
                        return false;
                    }
                }
                "nonland" => {
                    if card.is_land() {
                        return false;
                    }
                }
                "token" => {
                    if !card.is_token {
                        return false;
                    }
                }
                "nontoken" => {
                    if card.is_token {
                        return false;
                    }
                }
                "tapped" => {
                    if !card.tapped {
                        return false;
                    }
                }
                "untapped" => {
                    if card.tapped {
                        return false;
                    }
                }
                "startedtheturnuntapped" => {
                    if card.started_turn_tapped {
                        return false;
                    }
                }
                "startedtheturntapped" => {
                    if !card.started_turn_tapped {
                        return false;
                    }
                }
                "multicolor" => {
                    if !card.color.is_multicolor() {
                        return false;
                    }
                }
                "colorless" => {
                    if !card.color.is_colorless() {
                        return false;
                    }
                }
                "inzonebattlefield" => {
                    if card.zone != forge_foundation::ZoneType::Battlefield {
                        return false;
                    }
                }
                "inzonegraveyard" => {
                    if card.zone != forge_foundation::ZoneType::Graveyard {
                        return false;
                    }
                }
                "inzonehand" => {
                    if card.zone != forge_foundation::ZoneType::Hand {
                        return false;
                    }
                }
                "inzoneexile" => {
                    if card.zone != forge_foundation::ZoneType::Exile {
                        return false;
                    }
                }
                "damagedby" => {
                    // Check if this card was dealt damage by the source card this turn
                    if !card.damage_sources_this_turn.contains(&source.id) {
                        return false;
                    }
                }
                "enchantedby" | "attachedby" => {
                    // Check if source is attached to this card
                    if source.attached_to != Some(card.id) {
                        return false;
                    }
                }
                _ => {
                    // Check counters_GE/GT/LT/LE/EQ patterns like "counters_GE3_P1P1"
                    if sub.starts_with("counters_") {
                        if !check_counter_condition(sub, card) {
                            return false;
                        }
                    } else if let Some(rest) = sub_lower.strip_prefix("cmc") {
                        // CMC comparisons: cmcEQ1, cmcLE3, cmcGE5
                        if !check_cmc_condition(rest, card) {
                            return false;
                        }
                    } else if let Some(rest) = sub_lower.strip_prefix("power") {
                        // Power comparisons: powerLE2, powerGE3, etc.
                        if !check_power_condition(rest, card) {
                            return false;
                        }
                    } else if let Some(rest) = sub_lower.strip_prefix("toughness") {
                        // Toughness comparisons: toughnessLE2, toughnessGE3, etc.
                        if !check_toughness_condition(rest, card) {
                            return false;
                        }
                    } else if let Some(color) = Color::from_name(&sub_lower) {
                        // Color names: white, blue, black, red, green
                        if !card.color.has_color(color) {
                            return false;
                        }
                    } else if let Some(kw) = sub_lower.strip_prefix("with") {
                        // "withFlying", "withoutFlying", etc.
                        if kw.strip_prefix("out").is_some() {
                            // "withoutFlying" — card must NOT have this keyword
                            let kw_name = &sub[7..]; // original case
                            if card.has_keyword(kw_name) {
                                return false;
                            }
                        } else if !kw.is_empty() {
                            // "withFlying" — card must have this keyword
                            let kw_name = &sub[4..]; // original case
                            if !card.has_keyword(kw_name) {
                                return false;
                            }
                        }
                    } else if let Some(negated) = sub_lower.strip_prefix("non") {
                        // Negated qualifier: "nonBlack", "nonArtifact", "nonFlying", etc.
                        let should_negate = match negated {
                            "creature" => card.is_creature(),
                            "land" => card.is_land(),
                            "artifact" => card.type_line.is_artifact(),
                            "enchantment" => card.type_line.is_enchantment(),
                            "token" => card.is_token,
                            _ => {
                                if let Some(color) = Color::from_name(negated) {
                                    card.color.has_color(color)
                                } else {
                                    // nonSubtype: e.g., "nonHuman", "nonWall"
                                    card.has_subtype(
                                        &sub[3..], // use original case for subtype
                                    )
                                }
                            }
                        };
                        if should_negate {
                            return false;
                        }
                    } else if !sub.is_empty() {
                        // Color source check: "RedSource", "WhiteSource", "BlackSource", etc.
                        // Mirrors Java ForgeScript.cardStateHasProperty: strip "Source" suffix,
                        // then check card color. Also handles "nonRedSource" via the non- prefix above.
                        let color_name = if sub.ends_with("Source") {
                            &sub[..sub.len() - 6]
                        } else {
                            sub
                        };
                        if let Some(color) = Color::from_name(&color_name.to_lowercase()) {
                            if !card.color.has_color(color) {
                                return false;
                            }
                        } else if color_name.eq_ignore_ascii_case("Colorless") {
                            if !card.color.is_colorless() {
                                return false;
                            }
                        } else {
                            // Fall through: check as creature subtype (Wall, Zombie, etc.)
                            // Mirrors card_has_property behavior: unrecognized qualifiers
                            // are checked against the card's type_line subtypes.
                            if !card.has_subtype(sub) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }

    true
}

/// Check if a player matches a filter expression like "You", "Opponent", "Each".
pub fn matches_valid_player(filter: &str, player: PlayerId, source_controller: PlayerId) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }

    // Handle comma-separated alternatives
    if filter.contains(',') {
        return filter
            .split(',')
            .any(|part| matches_single_valid_player(part.trim(), player, source_controller));
    }

    matches_single_valid_player(filter, player, source_controller)
}

/// Convenience wrapper: None means "no filter" → always matches.
pub fn matches_valid_player_opt(
    filter: Option<&str>,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    match filter {
        None => true,
        Some(v) => matches_valid_player(v, player, source_controller),
    }
}

fn matches_single_valid_player(
    filter: &str,
    player: PlayerId,
    source_controller: PlayerId,
) -> bool {
    let filter_lower = filter.to_ascii_lowercase();
    match filter_lower.as_str() {
        "you" | "youctrl" => player == source_controller,
        "opponent" | "oppctrl" | "opponentctrl" => player != source_controller,
        "any" | "each" | "player" | "player.ingame" => true,
        // "Active" / "NonActive" would need turn info — not currently supported
        _ => true, // unknown filter, match all (permissive fallback)
    }
}

/// Check a counter condition like "counters_GE3_P1P1".
/// Format: counters_{op}{num}_{counter_type}
fn check_counter_condition(condition: &str, card: &Card) -> bool {
    use crate::ability::effects::parse_counter_type;
    let rest = &condition["counters_".len()..];
    if rest.len() < 3 {
        return true;
    }
    let op = &rest[..2];
    let after_op = &rest[2..];
    let (num_str, counter_type_str) = match after_op.find('_') {
        Some(idx) => (&after_op[..idx], &after_op[idx + 1..]),
        None => return true,
    };
    let threshold: i32 = num_str.parse().unwrap_or(0);
    let counter_type = parse_counter_type(counter_type_str);
    let count = card.counter_count(&counter_type);
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

/// Check a CMC condition like "cmcEQ1", "cmcLE3", "cmcGE5".
fn check_cmc_condition(rest: &str, card: &Card) -> bool {
    let cmc = card.mana_cost.cmc() as i32;
    if let Some(num_str) = rest.strip_prefix("eq") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc == n;
        }
    } else if let Some(num_str) = rest.strip_prefix("le") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc <= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ge") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc >= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("lt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc < n;
        }
    } else if let Some(num_str) = rest.strip_prefix("gt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc > n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ne") {
        if let Ok(n) = num_str.parse::<i32>() {
            return cmc != n;
        }
    }
    true // fallback: unknown format passes
}

/// Check a power condition like "LE2", "GE3", "EQ0".
fn check_power_condition(rest: &str, card: &Card) -> bool {
    let power = card.power();
    if let Some(num_str) = rest.strip_prefix("eq") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power == n;
        }
    } else if let Some(num_str) = rest.strip_prefix("le") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power <= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ge") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power >= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("lt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power < n;
        }
    } else if let Some(num_str) = rest.strip_prefix("gt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power > n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ne") {
        if let Ok(n) = num_str.parse::<i32>() {
            return power != n;
        }
    }
    true // fallback: unknown format passes
}

/// Check a toughness condition like "LE2", "GE3", "EQ0".
fn check_toughness_condition(rest: &str, card: &Card) -> bool {
    let toughness = card.toughness();
    if let Some(num_str) = rest.strip_prefix("eq") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness == n;
        }
    } else if let Some(num_str) = rest.strip_prefix("le") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness <= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ge") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness >= n;
        }
    } else if let Some(num_str) = rest.strip_prefix("lt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness < n;
        }
    } else if let Some(num_str) = rest.strip_prefix("gt") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness > n;
        }
    } else if let Some(num_str) = rest.strip_prefix("ne") {
        if let Ok(n) = num_str.parse::<i32>() {
            return toughness != n;
        }
    }
    true // fallback: unknown format passes
}

// ── Common requirement checks ───────────────────────────────────────────────
//
// These mirror Java's `CardTraitBase.meetsCommonRequirements()` — shared
// validation logic used by triggers, static abilities, replacement effects,
// and cost adjustment. Previously duplicated in 4+ locations.

/// Check the `IsPresent$` / `PresentCompare$` / `PresentPlayer$` /
/// `PresentZone$` parameter group.
///
/// Counts cards matching the `IsPresent$` filter in the specified zone for
/// the specified player(s), then compares the count against `PresentCompare$`.
///
/// Mirrors Java's `meetsCommonRequirements()` IsPresent block.
pub fn check_is_present(game: &GameState, params: &Params, source: &Card) -> bool {
    let Some(is_present) = params.get(keys::IS_PRESENT) else {
        return true; // no IsPresent param — passes
    };

    let present_compare = params.get(keys::PRESENT_COMPARE).unwrap_or("GE1");
    let present_player = params.get(keys::PRESENT_PLAYER).unwrap_or("Any");
    let present_zone = params
        .get(keys::PRESENT_ZONE)
        .and_then(parse_zone_name)
        .unwrap_or(ZoneType::Battlefield);

    let candidate_players: Vec<PlayerId> = match present_player {
        p if p.eq_ignore_ascii_case("You") => vec![source.controller],
        p if p.eq_ignore_ascii_case("Opponent") => vec![game.opponent_of(source.controller)],
        _ => game.players.iter().map(|p| p.id).collect(),
    };

    let mut count = 0i32;
    for pid in candidate_players {
        for &cid in game.cards_in_zone(present_zone, pid) {
            if matches_valid_card(is_present, game.card(cid), source) {
                count += 1;
            }
        }
    }

    compare_expr(count, present_compare)
}

/// Check the `CheckSVar$` / `SVarCompare$` parameter group.
///
/// Resolves the named SVar on the source card and compares its value.
/// Mirrors Java's `meetsCommonRequirements()` CheckSVar block.
pub fn check_svar_condition(game: &GameState, params: &Params, source: &Card) -> bool {
    let Some(check_name) = params.get(keys::CHECK_SVAR) else {
        return true;
    };
    let Some(compare) = params.get(keys::SVAR_COMPARE) else {
        return true;
    };

    // Resolve the SVar value — first check card SVars, then try direct parse.
    let raw_value = source
        .svars
        .get(check_name)
        .map(|s| s.as_str())
        .unwrap_or("0");
    let value = if raw_value.starts_with("Count$") {
        crate::svar::resolve_count_svar(raw_value, game, source.id, source.controller)
    } else {
        raw_value.parse::<i32>().unwrap_or(0)
    };

    compare_expr(value, compare)
}

/// Check a named `Check*SVar` / `*SVarCompare` pair.
///
/// This is the generalized form of `check_svar_condition`, used by
/// `StaticAbility.checkConditions()` for second/third/fourth SVar checks.
pub fn check_named_svar_condition(
    game: &GameState,
    params: &Params,
    source: &Card,
    check_key: &str,
    compare_key: &str,
) -> bool {
    let Some(check_name) = params.get(check_key) else {
        return true;
    };
    let compare = params.get(compare_key).unwrap_or("GE1");

    let raw_value = source
        .svars
        .get(check_name)
        .map(|s| s.as_str())
        .unwrap_or("0");
    let value = if raw_value.starts_with("Count$") {
        crate::svar::resolve_count_svar(raw_value, game, source.id, source.controller)
    } else {
        raw_value.parse::<i32>().unwrap_or(0)
    };

    compare_expr(value, compare)
}

/// Check the `Condition$` parameter for game-state conditions.
///
/// Supports: PlayerTurn, NotPlayerTurn, Metalcraft, Delirium.
/// Mirrors Java's `meetsCommonRequirements()` condition checks.
pub fn check_condition(game: &GameState, params: &Params, source: &Card) -> bool {
    let Some(condition) = params.get(keys::CONDITION) else {
        return true;
    };
    match condition {
        "PlayerTurn" => game.active_player() == source.controller,
        "NotPlayerTurn" => game.active_player() != source.controller,
        "Threshold" => game.player_has_threshold(source.controller),
        "Hellbent" => game.player_has_hellbent(source.controller),
        "Metalcraft" => game.player_has_metalcraft(source.controller),
        "Delirium" => game.player_has_delirium(source.controller),
        "Ferocious" => game.player_has_ferocious(source.controller),
        "Desert" => game.player_has_desert(source.controller),
        "Blessing" => game.player_has_blessing(source.controller),
        "Monarch" => game.monarch == Some(source.controller),
        "Night" => game.is_night,
        "FatefulHour" => game.player(source.controller).life <= 5,
        _ => true, // unknown condition — permissive fallback
    }
}

/// Convenience: run all common requirement checks from a `Params` instance.
///
/// Checks `IsPresent$`, `CheckSVar$`, and `Condition$` in sequence.
/// Returns `false` if any check fails.
///
/// Mirrors Java's `CardTraitBase.meetsCommonRequirements()`.
pub fn meets_common_requirements(game: &GameState, params: &Params, source: &Card) -> bool {
    check_is_present(game, params, source)
        && check_svar_condition(game, params, source)
        && check_condition(game, params, source)
}

/// Parse a zone name string into ZoneType.
fn parse_zone_name(name: &str) -> Option<ZoneType> {
    match name.to_ascii_lowercase().as_str() {
        "battlefield" => Some(ZoneType::Battlefield),
        "graveyard" => Some(ZoneType::Graveyard),
        "hand" => Some(ZoneType::Hand),
        "library" => Some(ZoneType::Library),
        "exile" => Some(ZoneType::Exile),
        "command" => Some(ZoneType::Command),
        _ => None,
    }
}
