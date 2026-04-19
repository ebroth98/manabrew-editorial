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
use forge_foundation::mana::ManaAtom;
use forge_foundation::ZoneType;

use crate::card::Card;
use crate::core::HasSVars;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;
use crate::parsing::Params;

fn requirement_controller(game: &GameState, source: &Card) -> PlayerId {
    let mut controller = source.controller;

    // Java parity: during trigger resolution, use the resolving trigger's
    // activating player rather than the host card's current controller.
    if !game.stack.is_empty()
        && game.stack.is_resolving()
        && game.stack.cur_resolving_card() == Some(source.id)
    {
        if let Some(entry) = game.stack.peek() {
            if entry.spell_ability.is_trigger {
                controller = entry.spell_ability.activating_player;
            }
        }
    }

    controller
}

fn requirement_amount(
    source: &Card,
    svar_source: &dyn HasSVars,
    expr: &str,
    game: &GameState,
) -> i32 {
    let raw_value = svar_source
        .get_svar(expr)
        .or_else(|| source.get_s_var(expr))
        .unwrap_or(expr)
        .trim();

    if let Ok(n) = raw_value.parse::<i32>() {
        return n;
    }
    if let Some(stripped) = raw_value.strip_prefix('+') {
        if let Ok(n) = stripped.parse::<i32>() {
            return n;
        }
    }
    if let Some(stripped) = raw_value.strip_prefix('-') {
        return -requirement_amount(source, svar_source, stripped.trim(), game);
    }

    if raw_value.starts_with("Count$") {
        return crate::svar::resolve_count_svar(raw_value, game, source.id, source.controller);
    }

    let mut sa = crate::spellability::SpellAbility::new_simple(
        Some(source.id),
        requirement_controller(game, source),
        &format!("DB$ Internal | Amount$ {raw_value}"),
    );
    sa.params.put("Amount".to_string(), raw_value.to_string());
    let resolved = crate::svar::resolve_numeric_svar(game, &sa, "Amount", i32::MIN);
    if resolved != i32::MIN {
        return resolved;
    }

    0
}

fn compare_requirement_amount(
    source: &Card,
    svar_source: &dyn HasSVars,
    compare: &str,
    game: &GameState,
    left: i32,
) -> bool {
    let operator = compare.get(..compare.len().min(2)).unwrap_or("GE");
    let operand_expr = compare.get(compare.len().min(2)..).unwrap_or("1");
    let operand = requirement_amount(source, svar_source, operand_expr, game);
    compare_expr(left, &format!("{operator}{operand}"))
}

fn check_named_boolean_param(params: &Params, key: &str, actual: bool) -> bool {
    let Some(value) = params.get(key) else {
        return true;
    };
    value.eq_ignore_ascii_case("True") == actual
}

fn player_life_for_requirement(game: &GameState, source: &Card, who: &str) -> i32 {
    let controller = requirement_controller(game, source);
    match who {
        "You" => game.player(controller).life,
        "OpponentSmallest" => game
            .alive_players()
            .into_iter()
            .filter(|&pid| pid != controller)
            .map(|pid| game.player(pid).life)
            .min()
            .unwrap_or(1),
        "OpponentGreatest" => game
            .alive_players()
            .into_iter()
            .filter(|&pid| pid != controller)
            .map(|pid| game.player(pid).life)
            .max()
            .unwrap_or(1),
        "ActivePlayer" => game.player(game.active_player()).life,
        _ => 1,
    }
}

fn collect_present_cards(
    game: &GameState,
    source: &Card,
    defined: Option<&str>,
    present_player: &str,
    present_zone: ZoneType,
) -> Vec<CardId> {
    if let Some(defined) = defined {
        return crate::ability::ability_utils::get_defined_cards(
            game,
            Some(source.id),
            defined,
            Some(requirement_controller(game, source)),
        );
    }

    let controller = requirement_controller(game, source);
    let mut cards = Vec::new();

    if present_player.eq_ignore_ascii_case("You") || present_player.eq_ignore_ascii_case("Any") {
        cards.extend(game.cards_in_zone(present_zone, controller).iter().copied());
    }
    if present_player.eq_ignore_ascii_case("Opponent") || present_player.eq_ignore_ascii_case("Any")
    {
        for pid in game.alive_players() {
            if pid != controller {
                cards.extend(game.cards_in_zone(present_zone, pid).iter().copied());
            }
        }
    }

    cards
}

fn paying_color_count(paying_mana_to_cast: &[u16], color_mask: u16) -> usize {
    paying_mana_to_cast
        .iter()
        .filter(|&&atom| atom == color_mask)
        .count()
}

fn has_all_spent_colors(colors_spent_to_cast: u16, colors: u16) -> bool {
    colors != 0 && (colors_spent_to_cast & colors) == colors
}

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

    // Comma-separated = OR conditions.
    // Each comma-delimited part is a separate filter; the card matches if ANY part matches.
    // Parts may contain dots (e.g. "Card.Self,Elemental.Other+YouCtrl").
    if valid.contains(',') {
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
                "strictlyself" => {
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
                "legendary" => {
                    if !card.type_line.is_legendary() {
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
                "equippedby" | "enchantedby" | "attachedby" => {
                    // Check if source is attached to this card
                    if source.attached_to != Some(card.id) {
                        return false;
                    }
                }
                "wascast" => {
                    // Mirrors Java CardProperty.java:1923-1926 — card must have been
                    // cast (not put onto the battlefield by some other means).
                    if !card.was_cast() {
                        return false;
                    }
                }
                "wascastbyyou" => {
                    // Mirrors Java CardProperty.java:1923-1929: wasCast AND the
                    // spell's activating player equals source's controller.
                    // Rust doesn't track castSA.activatingPlayer separately; the
                    // card's controller at ETB time equals the caster for normal
                    // casts, which covers Sunderflock-style triggers.
                    if !card.was_cast() || card.controller != source.controller {
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
                    } else if sub_lower == "chosentype" {
                        // "ChosenType" — card must have the source card's chosen
                        // creature type. Changeling counts as all creature types.
                        // Mirrors Java CardTraitBase.isValid() ChosenType path.
                        let matches = if let Some(ref ct) = source.chosen_type {
                            card.type_line.has_subtype(ct) || card.has_keyword("Changeling")
                        } else {
                            false
                        };
                        if !matches {
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

/// Mirrors Java's `CardTraitBase.matchesValid(Object, String[], Card, Player)`.
///
/// Java uses polymorphic dispatch via `GameObject.isValid()` — both Card and
/// Player implement it. In Rust, we take both as Options and try card first,
/// then player, mirroring the `instanceof` chain in Java.
///
/// This eliminates the need for callers to guess whether a filter string can
/// match a player (the old `filter_can_match_player` heuristic).
pub fn matches_valid(
    filter: &str,
    card: Option<&Card>,
    player: Option<PlayerId>,
    source: &Card,
    source_controller: PlayerId,
) -> bool {
    if let Some(card) = card {
        matches_valid_card(filter, card, source)
    } else if let Some(player) = player {
        matches_valid_player(filter, player, source_controller)
    } else {
        false
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
pub fn check_is_present(
    game: &GameState,
    params: &Params,
    source: &Card,
    svar_source: &dyn HasSVars,
) -> bool {
    let Some(is_present) = params.get(keys::IS_PRESENT) else {
        return true; // no IsPresent param — passes
    };

    let present_compare = params.get(keys::PRESENT_COMPARE).unwrap_or("GE1");
    let present_player = params.get(keys::PRESENT_PLAYER).unwrap_or("Any");
    let present_zone = params
        .get(keys::PRESENT_ZONE)
        .and_then(parse_zone_name)
        .unwrap_or(ZoneType::Battlefield);
    let present_defined = params.get("PresentDefined");

    let count = collect_present_cards(game, source, present_defined, present_player, present_zone)
        .into_iter()
        .filter(|&cid| matches_valid_card(is_present, game.card(cid), source))
        .count() as i32;

    compare_requirement_amount(source, svar_source, present_compare, game, count)
}

/// Check the `CheckSVar$` / `SVarCompare$` parameter group.
///
/// Resolves the named SVar on the source card and compares its value.
/// Mirrors Java's `meetsCommonRequirements()` CheckSVar block.
pub fn check_svar_condition(
    game: &GameState,
    params: &Params,
    source: &Card,
    svar_source: &dyn HasSVars,
) -> bool {
    let Some(check_name) = params.get(keys::CHECK_SVAR) else {
        return true;
    };
    let compare = params.get(keys::SVAR_COMPARE).unwrap_or("GE1");

    compare_svar(game, source, svar_source, check_name, compare)
        && check_named_svar_condition(
            game,
            params,
            source,
            svar_source,
            "CheckSecondSVar",
            "SecondSVarCompare",
        )
}

/// Check a named `Check*SVar` / `*SVarCompare` pair.
///
/// This is the generalized form of `check_svar_condition`, used by
/// `StaticAbility.checkConditions()` for second/third/fourth SVar checks.
pub fn check_named_svar_condition(
    game: &GameState,
    params: &Params,
    source: &Card,
    svar_source: &dyn HasSVars,
    check_key: &str,
    compare_key: &str,
) -> bool {
    let Some(check_name) = params.get(check_key) else {
        return true;
    };
    let compare = params.get(compare_key).unwrap_or("GE1");

    compare_svar(game, source, svar_source, check_name, compare)
}

fn compare_svar(
    game: &GameState,
    source: &Card,
    svar_source: &dyn HasSVars,
    check_name: &str,
    compare: &str,
) -> bool {
    let value = requirement_amount(source, svar_source, check_name, game);
    compare_requirement_amount(source, svar_source, compare, game, value)
}

fn resolve_svar_requirement_value(
    source: &Card,
    svar_source: &dyn HasSVars,
    expr: &str,
    game: &GameState,
) -> i32 {
    requirement_amount(source, svar_source, expr, game)
}

/// Check the `Condition$` parameter for game-state conditions.
///
/// Supports: PlayerTurn, NotPlayerTurn, Metalcraft, Delirium.
/// Mirrors Java's `meetsCommonRequirements()` condition checks.
pub fn check_condition(game: &GameState, params: &Params, source: &Card) -> bool {
    let Some(condition) = params.get(keys::CONDITION) else {
        return true;
    };
    let controller = requirement_controller(game, source);
    match condition {
        "PlayerTurn" => game.active_player() == controller,
        "NotPlayerTurn" => game.active_player() != controller,
        "Threshold" => game.player_has_threshold(controller),
        "Hellbent" => game.player_has_hellbent(controller),
        "Metalcraft" => game.player_has_metalcraft(controller),
        "Delirium" => game.player_has_delirium(controller),
        "Ferocious" => game.player_has_ferocious(controller),
        "Desert" => game.player_has_desert(controller),
        "Blessing" => game.player_has_blessing(controller),
        "Monarch" => game.monarch == Some(controller),
        "Night" => game.is_night,
        "FatefulHour" => game.player(controller).life <= 5,
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
    meets_common_requirements_with_svars(game, params, source, source)
}

pub fn meets_common_requirements_with_svars(
    game: &GameState,
    params: &Params,
    source: &Card,
    svar_source: &dyn HasSVars,
) -> bool {
    let controller = requirement_controller(game, source);

    if !check_named_boolean_param(params, "Metalcraft", game.player_has_metalcraft(controller)) {
        return false;
    }
    if !check_named_boolean_param(params, "Delirium", game.player_has_delirium(controller)) {
        return false;
    }
    if !check_named_boolean_param(params, "Threshold", game.player_has_threshold(controller)) {
        return false;
    }
    if !check_named_boolean_param(params, "Hellbent", game.player_has_hellbent(controller)) {
        return false;
    }
    if !check_named_boolean_param(
        params,
        "Bloodthirst",
        game.player_has_bloodthirst(controller),
    ) {
        return false;
    }
    if !check_named_boolean_param(params, "FatefulHour", game.player(controller).life <= 5) {
        return false;
    }
    if !check_named_boolean_param(params, "Monarch", game.monarch == Some(controller)) {
        return false;
    }
    if let Some(revolt) = params.get("Revolt") {
        if revolt.eq_ignore_ascii_case("True") != game.player_has_revolt(controller) {
            return false;
        } else if revolt.eq_ignore_ascii_case("None")
            && game
                .alive_players()
                .into_iter()
                .any(|pid| game.player_has_revolt(pid))
        {
            return false;
        }
    }
    if !check_named_boolean_param(params, "Desert", game.player_has_desert(controller)) {
        return false;
    }
    if !check_named_boolean_param(params, "Blessing", game.player_has_blessing(controller)) {
        return false;
    }

    if let Some(day_time) = params.get("DayTime") {
        if day_time.eq_ignore_ascii_case("Day") {
            if !game.is_day() {
                return false;
            }
        } else if day_time.eq_ignore_ascii_case("Night") {
            if !game.is_night {
                return false;
            }
        } else if day_time.eq_ignore_ascii_case("Neither") {
            if !game.is_neither_day_nor_night() {
                return false;
            }
        }
    }

    if let Some(adamant) = params.get("Adamant") {
        let color_mask = ManaAtom::from_name(&adamant.to_ascii_lowercase());
        if adamant.eq_ignore_ascii_case("Any") {
            let has_three = [
                ManaAtom::WHITE,
                ManaAtom::BLUE,
                ManaAtom::BLACK,
                ManaAtom::RED,
                ManaAtom::GREEN,
            ]
            .into_iter()
            .any(|mask| paying_color_count(&source.paying_mana_to_cast, mask) >= 3);
            if !has_three {
                return false;
            }
        } else if paying_color_count(&source.paying_mana_to_cast, color_mask) < 3 {
            return false;
        }
    }

    if let Some(life_total) = params.get("LifeTotal") {
        let compare = params.get("LifeAmount").unwrap_or("GE1");
        let life = player_life_for_requirement(game, source, life_total);
        if !compare_requirement_amount(source, svar_source, compare, game, life) {
            return false;
        }
    }

    if !check_is_present(game, params, source, svar_source) {
        return false;
    }

    if let Some(is_present) = params.get("IsPresent2") {
        let present_compare = params.get("PresentCompare2").unwrap_or("GE1");
        let present_player = params.get("PresentPlayer2").unwrap_or("Any");
        let present_zone = params
            .get("PresentZone2")
            .and_then(parse_zone_name)
            .unwrap_or(ZoneType::Battlefield);
        let count = collect_present_cards(game, source, None, present_player, present_zone)
            .into_iter()
            .filter(|&cid| matches_valid_card(is_present, game.card(cid), source))
            .count() as i32;
        if !compare_requirement_amount(source, svar_source, present_compare, game, count) {
            return false;
        }
    }

    if let Some(defined_players) = params.get("CheckDefinedPlayer") {
        let players = crate::ability::ability_utils::get_defined_players(
            game,
            Some(source.id),
            defined_players,
            Some(controller),
        );
        let compare = params.get("DefinedPlayerCompare").unwrap_or("GE1");
        if !compare_requirement_amount(source, svar_source, compare, game, players.len() as i32) {
            return false;
        }
    }

    if !check_svar_condition(game, params, source, svar_source) {
        return false;
    }

    if let Some(mana_spent) = params.get("ManaSpent") {
        let colors = ManaAtom::from_name(&mana_spent.to_ascii_lowercase());
        if !has_all_spent_colors(source.colors_spent_to_cast, colors) {
            return false;
        }
    }
    if let Some(mana_not_spent) = params.get("ManaNotSpent") {
        let colors = ManaAtom::from_name(&mana_not_spent.to_ascii_lowercase());
        if has_all_spent_colors(source.colors_spent_to_cast, colors) {
            return false;
        }
    }

    if params.has("WerewolfTransformCondition")
        && !game.stack.get_spells_cast_last_turn().is_empty()
    {
        return false;
    }
    if params.has("WerewolfUntransformCondition") {
        let cast_last_turn = game.stack.get_spells_cast_last_turn();
        let mut condition_met = false;
        for pid in game.alive_players() {
            let count = cast_last_turn
                .iter()
                .filter(|&&cid| game.card(cid).controller == pid)
                .count();
            if count > 1 {
                condition_met = true;
                break;
            }
        }
        if !condition_met {
            return false;
        }
    }

    if let Some(class_level) = params.get("ClassLevel") {
        let min = class_level.parse::<i32>().unwrap_or(0);
        if source.class_level < min {
            return false;
        }
    }

    check_condition(game, params, source)
}

/// Parse a zone name string into ZoneType.
fn parse_zone_name(name: &str) -> Option<ZoneType> {
    ZoneType::from_str_compat(name)
}
