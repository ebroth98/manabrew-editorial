//! AbilityUtils — utility functions for ability resolution.
//!
//! Mirrors Java's `AbilityUtils.java`.
//! This is the single source of truth for helper functions used across effects.
//! The `effects::helpers` module re-exports everything from here for backward
//! compatibility.

use forge_foundation::{ColorSet, ZoneType};

use crate::card::filter_constants as fc;
use crate::card::{CardInstance, CounterType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

// ── Defined$ Card Resolution ─────────────────────────────────────────

/// Resolve `Defined$` strings to a list of card IDs.
/// Mirrors Java's `AbilityUtils.getDefinedCards()`.
pub fn get_defined_cards(
    game: &GameState,
    host_card: Option<CardId>,
    defined: &str,
    _activating_player: Option<PlayerId>,
) -> Vec<CardId> {
    match defined {
        "Self" | "CARDNAME" => host_card.into_iter().collect(),
        "Remembered" => {
            if let Some(src) = host_card {
                game.card(src).remembered_cards.clone()
            } else {
                Vec::new()
            }
        }
        "Imprinted" => {
            if let Some(src) = host_card {
                game.card(src).imprinted_cards.clone()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// Resolve `Defined$` strings to a list of player IDs.
/// Mirrors Java's `AbilityUtils.getDefinedPlayers()`.
pub fn get_defined_players(
    game: &GameState,
    _host_card: Option<CardId>,
    defined: &str,
    activating_player: Option<PlayerId>,
) -> Vec<PlayerId> {
    if let Some(player) = activating_player {
        resolve_defined_players(defined, player, game)
    } else {
        Vec::new()
    }
}

/// Calculate a numeric amount from a parameter string.
/// Handles simple integers and "X" references.
///
/// For the full implementation with SVar support, use
/// `svar::resolve_numeric_svar()`.
pub fn calculate_amount(value: &str) -> i32 {
    value.parse::<i32>().unwrap_or(0)
}

// ── Parameter Parsing ────────────────────────────────────────────────

/// Parse a numeric parameter from an ability string (e.g. "NumAtt$ 3" → 3).
pub fn parse_param(ability: &str, prefix: &str) -> Option<i32> {
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix(prefix) {
            if let Ok(n) = val.trim().parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Parse NumDmg$ value from an ability string.
pub fn parse_num_dmg(ability: &str) -> i32 {
    parse_param(ability, "NumDmg$ ").unwrap_or(0)
}

// ── Defined$ Player Resolution ───────────────────────────────────────

/// Resolve a Defined$ parameter to a player ID.
/// Mirrors Java's AbilityUtils.getDefinedPlayers().
///
/// Handles both bare names ("Opponent") and prefixed forms ("Player.Opponent")
/// used by cards like Guttersnipe: `Defined$ Player.Opponent`.
pub fn resolve_defined_player(
    defined: &str,
    controller: PlayerId,
    game: &GameState,
) -> Option<PlayerId> {
    // Strip "Player." prefix if present (e.g. "Player.Opponent" → "Opponent")
    let key = defined.strip_prefix("Player.").unwrap_or(defined);
    match key {
        "You" => Some(controller),
        "Opponent" | "OpponentCtrl" => {
            let opp = game.opponent_of(controller);
            Some(opp)
        }
        "DefendingPlayer" | "TriggeredDefendingPlayer" => Some(game.opponent_of(controller)),
        _ => None,
    }
}

/// Resolve a Defined$ parameter to a player ID with spell/trigger context.
/// Mirrors Java's Triggered* player resolution for trigger SVar chains.
pub fn resolve_defined_player_with_sa(
    defined: &str,
    sa: &SpellAbility,
    controller: PlayerId,
    game: &GameState,
) -> Option<PlayerId> {
    let key = defined.strip_prefix("Player.").unwrap_or(defined);
    match key {
        "TriggeredPlayer" | "TargetedPlayer" => sa.target_chosen.target_player,
        "DefendingPlayer" | "TriggeredDefendingPlayer" => sa
            .target_chosen
            .target_player
            .or_else(|| Some(game.opponent_of(controller))),
        "TriggeredController" => sa
            .trigger_source
            .map(|cid| game.card(cid).controller)
            .or(sa.target_chosen.target_player),
        "TargetedController" => sa
            .target_chosen
            .target_card
            .map(|cid| game.card(cid).controller)
            .or(sa.target_chosen.target_player),
        _ => resolve_defined_player(key, controller, game),
    }
}

/// Resolve a Defined$ parameter to a list of player IDs.
/// Supports "You", "Opponent", "Each"/"All"/"Player" (all alive players).
/// Mirrors Java's AbilityUtils.getDefinedPlayers() for multi-player resolution.
pub fn resolve_defined_players(
    defined: &str,
    controller: PlayerId,
    game: &GameState,
) -> Vec<PlayerId> {
    match defined {
        "You" => vec![controller],
        "Opponent" | "OpponentCtrl" => vec![game.opponent_of(controller)],
        "DefendingPlayer" | "TriggeredDefendingPlayer" => vec![game.opponent_of(controller)],
        "Each" | "All" | "Player" => game.alive_players(),
        _ => {
            // Fall back to single-player resolution
            if let Some(pid) = resolve_defined_player(defined, controller, game) {
                vec![pid]
            } else {
                vec![controller]
            }
        }
    }
}

// ── Counter Type Parsing ─────────────────────────────────────────────

/// Parse a counter type string to CounterType enum (case-insensitive).
/// Unknown types produce `CounterType::Named(UPPER)` instead of silently
/// falling back to P1P1, so cards like Stocking the Pantry get the correct
/// SUPPLY counters.
pub fn parse_counter_type(s: &str) -> CounterType {
    match s.to_uppercase().as_str() {
        "P1P1" | "+1/+1" => CounterType::P1P1,
        "M1M1" | "-1/-1" => CounterType::M1M1,
        "LOYALTY" => CounterType::Loyalty,
        "CHARGE" => CounterType::Charge,
        "QUEST" => CounterType::Quest,
        "STUDY" => CounterType::Study,
        "AGE" => CounterType::Age,
        "FADE" => CounterType::Fade,
        "TIME" => CounterType::Time,
        "DEPLETION" => CounterType::Depletion,
        "STORAGE" => CounterType::Storage,
        "MINING" => CounterType::Mining,
        "BRICK" => CounterType::Brick,
        "LEVEL" => CounterType::Level,
        "LORE" => CounterType::Lore,
        "PAGE" => CounterType::Page,
        "DREAM" => CounterType::Dream,
        other => CounterType::Named(other.to_string()),
    }
}

// ── Zone Type Parsing ────────────────────────────────────────────────

/// Parse a zone name string to ZoneType.
pub fn parse_zone_type(s: &str) -> Option<ZoneType> {
    match s.trim() {
        "Battlefield" => Some(ZoneType::Battlefield),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" | "Deck" => Some(ZoneType::Library),
        "Exile" => Some(ZoneType::Exile),
        "Command" => Some(ZoneType::Command),
        _ => None,
    }
}

// ── ValidCards$ Matching ─────────────────────────────────────────────

/// Full ValidCards$ filter matching with controller and keyword qualifier support.
///
/// This is the preferred function for mass effects (DestroyAll, DamageAll, etc.)
/// because it handles `YouCtrl`, `OppCtrl`, `withFlying`, and color (`nonBlack`)
/// qualifiers in addition to card types.
///
/// `activating_player` is the player who cast/activated the ability; used to
/// resolve `YouCtrl` / `OppCtrl` qualifiers.
///
/// Mirrors Java's `CardLists.getValidCards()` + `CardProperty.cardHasProperty()`.
pub fn matches_valid_cards(card: &CardInstance, filter: &str, activating_player: PlayerId) -> bool {
    if filter.is_empty() || filter == fc::CARD {
        return true;
    }

    // Comma-separated = OR conditions (e.g. "Creature.attacking Opponent, Creature.attacking Planeswalker.OppCtrl")
    if filter.contains(", ") {
        return filter
            .split(", ")
            .any(|part| matches_valid_cards_single(card, part.trim(), activating_player));
    }

    matches_valid_cards_single(card, filter, activating_player)
}

fn matches_valid_cards_single(
    card: &CardInstance,
    filter: &str,
    activating_player: PlayerId,
) -> bool {
    let parts: Vec<&str> = filter.split('.').collect();
    let type_part = parts[0];

    // ── Type check ──────────────────────────────────────────────────────────
    let type_matches = match type_part {
        fc::CREATURE => card.is_creature(),
        fc::LAND => card.is_land(),
        fc::ARTIFACT => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(fc::ARTIFACT)),
        fc::ENCHANTMENT => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(fc::ENCHANTMENT)),
        fc::PLANESWALKER => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(fc::PLANESWALKER)),
        fc::INSTANT => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(fc::INSTANT)),
        fc::SORCERY => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(fc::SORCERY)),
        fc::PERMANENT | fc::CARD => true,
        _ => true, // Unknown type — match everything
    };
    if !type_matches {
        return false;
    }

    // ── Qualifier checks (dot-separated after the type) ─────────────────────
    // Handle compound "+" syntax (e.g. "YouCtrl+nonBlack", "Self+kicked")
    for &qualifier in &parts[1..] {
        let sub_parts: Vec<&str> = qualifier.split('+').collect();
        for sub in &sub_parts {
            if !matches_valid_cards_qualifier(card, sub, activating_player) {
                return false;
            }
        }
    }
    true
}

fn matches_valid_cards_qualifier(
    card: &CardInstance,
    qualifier: &str,
    activating_player: PlayerId,
) -> bool {
    match qualifier {
        fc::YOU_CTRL => card.controller == activating_player,
        fc::OPP_CTRL => card.controller != activating_player,
        fc::BASIC => card.type_line.is_basic(),
        fc::KICKED => card.kicked,
        fc::WITH_FLYING => {
            card.keywords.contains_string_ignore_case("Flying")
                || card.granted_keywords.contains_string_ignore_case("Flying")
        }
        _ => {
            // "attacking Opponent" / "attacking Planeswalker" — space-separated combat qualifier
            if let Some(target) = qualifier.strip_prefix("attacking ") {
                let attacking = card.attacking_player;
                match target {
                    "Opponent" => match attacking {
                        Some(def) => def != activating_player,
                        None => false,
                    },
                    // "attacking Planeswalker" — only true if attacking a planeswalker (not a player).
                    // Currently combat only tracks player targets, so this is always false.
                    "Planeswalker" => false,
                    _ => attacking.is_some(), // any attack target
                }
            }
            // Color filters: "nonBlack", "nonRed", "nonWhite", etc.
            else {
                let lower = qualifier.to_ascii_lowercase();
                if let Some(color_name) = lower.strip_prefix("non") {
                    let excluded = ColorSet::from_names(color_name);
                    !card.color.shares_color_with(excluded)
                } else {
                    // Unknown qualifier — match everything (forward-compatible)
                    true
                }
            }
        }
    }
}

// ── ChangeType$ Matching ─────────────────────────────────────────────

/// Check if a card matches a ChangeType$ / ValidCards$ filter string.
///
/// `source_chosen_colors` should be the `chosen_colors` from the source card
/// of the spell/ability (for `ChosenColor` qualifier support). Pass `&[]` when
/// no source card context is available.
pub fn matches_change_type(
    card: &CardInstance,
    change_type: &str,
    source_chosen_colors: &[String],
) -> bool {
    if change_type.is_empty() {
        return true;
    }

    // Handle semicolon-separated alternatives (OR).
    // E.g. "Artifact;Creature" means Artifact OR Creature.
    // Mirrors Java's CardLists.getValidCards() which splits on "," and ";".
    if change_type.contains(';') {
        return change_type
            .split(';')
            .any(|alt| matches_change_type(card, alt.trim(), source_chosen_colors));
    }

    // Handle comma-separated alternatives (OR).
    // E.g. "Artifact,Creature" means Artifact OR Creature.
    if change_type.contains(',') {
        return change_type
            .split(',')
            .any(|alt| matches_change_type(card, alt.trim(), source_chosen_colors));
    }

    let parts: Vec<&str> = change_type.split('.').collect();
    let type_part = parts[0];

    let type_matches = match type_part {
        fc::LAND => card.is_land(),
        fc::CREATURE => card.is_creature(),
        fc::ARTIFACT => card.type_line.is_artifact(),
        fc::ENCHANTMENT => card.type_line.is_enchantment(),
        fc::INSTANT => card.type_line.is_instant(),
        fc::SORCERY => card.type_line.is_sorcery(),
        fc::PLANESWALKER => card.type_line.is_planeswalker(),
        fc::PERMANENT => card.is_permanent(),
        fc::CARD => true,
        // Support land-subtype selectors used in tutor scripts
        // (e.g. "Forest.Basic", "Plains.Basic").
        "Plains" | "Island" | "Swamp" | "Mountain" | "Forest" => card
            .type_line
            .subtypes
            .iter()
            .any(|st| st.eq_ignore_ascii_case(type_part)),
        _ => card.type_line.has_subtype(type_part),
    };

    if !type_matches {
        return false;
    }

    for &qualifier in &parts[1..] {
        match qualifier {
            fc::BASIC => {
                if !card.type_line.is_basic() {
                    return false;
                }
            }
            fc::NON_LAND => {
                if card.is_land() {
                    return false;
                }
            }
            fc::ATTACKING => {
                if card.attacking_player.is_none() {
                    return false;
                }
            }
            "ChosenColor" => {
                if source_chosen_colors.is_empty() {
                    return false;
                }
                let mut chosen_set = ColorSet::COLORLESS;
                for name in source_chosen_colors {
                    chosen_set = chosen_set.union(ColorSet::from_names(name));
                }
                if !card.color.shares_color_with(chosen_set) {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

// ── Madness Discard Helper ───────────────────────────────────────────

/// Handle a card being discarded, applying the Madness replacement effect
/// if applicable. If the card has Madness, it goes to exile (marked with
/// `KEYWORD_MADNESS_EXILED`); otherwise it goes to the graveyard.
///
/// Also registers zone triggers and fires the Discarded trigger.
/// Mirrors Java's Madness replacement effect + discard trigger flow.
pub fn discard_with_madness_replacement(
    game: &mut GameState,
    trigger_handler: &mut crate::trigger::handler::TriggerHandler,
    card_id: CardId,
    discard_player: PlayerId,
) {
    let owner = game.card(card_id).owner;
    let has_madness = game.card(card_id).get_madness_cost().is_some();

    if has_madness {
        game.move_card(card_id, ZoneType::Exile, owner);
        trigger_handler.register_active_trigger(game, card_id);
        crate::ability::effects::zone_triggers::emit_zone_trigger(
            trigger_handler,
            card_id,
            ZoneType::Hand,
            ZoneType::Exile,
        );
        game.card_mut(card_id)
            .granted_keywords
            .add(crate::card::KEYWORD_MADNESS_EXILED);
    } else {
        game.move_card(card_id, ZoneType::Graveyard, owner);
        trigger_handler.register_active_trigger(game, card_id);
        crate::ability::effects::zone_triggers::emit_zone_trigger(
            trigger_handler,
            card_id,
            ZoneType::Hand,
            ZoneType::Graveyard,
        );
    }

    trigger_handler.run_trigger(
        crate::event::TriggerType::Discarded,
        crate::event::RunParams {
            card: Some(card_id),
            player: Some(discard_player),
            ..Default::default()
        },
        false,
    );
}

/// Remove the `MadnessExiled` marker from a card's granted keywords.
pub fn remove_madness_exiled_marker(card: &mut CardInstance) {
    card.granted_keywords
        .retain(|k| k != crate::card::KEYWORD_MADNESS_EXILED);
}
