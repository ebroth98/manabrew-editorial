//! SpellAbilityEffect — base trait and utility free functions for effects.
//!
//! Mirrors Java's `SpellAbilityEffect.java`.
//! In Java this is an abstract class with many protected static helpers;
//! in Rust we keep the trait for interface parity and provide the utility
//! methods as free functions that take `(&GameState, &SpellAbility)`.

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

use super::ability_utils;
use super::effects::EffectContext;

/// Base trait for all spell ability effect implementations.
///
/// Mirrors Java's abstract `SpellAbilityEffect` class.
/// Each effect type provides a `resolve` implementation that performs
/// the actual game-state mutation.
pub trait SpellAbilityEffect {
    /// Resolve this effect for the given spell ability.
    fn resolve(&self, ctx: &mut EffectContext, sa: &SpellAbility);

    /// Return the stack description for this effect.
    /// Defaults to the spell ability's own description.
    fn get_stack_description(&self, sa: &SpellAbility) -> String {
        sa.ability_text.clone()
    }

    /// Build/configure the spell ability after construction.
    /// Default is a no-op; some effects override this to add parameters.
    fn build_spell_ability(&self, _sa: &mut SpellAbility) {}
}

// ── Utility free functions mirroring Java's SpellAbilityEffect helpers ──

/// Get target cards for a spell ability.
/// If the SA uses targeting, returns the chosen target card(s).
/// Otherwise, resolves the `Defined$` parameter (defaulting to "Self").
///
/// Mirrors Java's `SpellAbilityEffect.getTargetCards(sa)`.
pub fn get_target_cards(game: &GameState, sa: &SpellAbility) -> Vec<CardId> {
    get_cards(game, sa, false, "Defined")
}

/// Get defined cards, falling back to targeted cards if no `Defined$` param.
///
/// Mirrors Java's `SpellAbilityEffect.getDefinedCardsOrTargeted(sa)`.
pub fn get_defined_cards_or_targeted(game: &GameState, sa: &SpellAbility) -> Vec<CardId> {
    get_cards(game, sa, true, "Defined")
}

/// Get defined cards with a custom param name, falling back to targeted.
///
/// Mirrors Java's `SpellAbilityEffect.getDefinedCardsOrTargeted(sa, definedParam)`.
pub fn get_defined_cards_or_targeted_param(
    game: &GameState,
    sa: &SpellAbility,
    defined_param: &str,
) -> Vec<CardId> {
    get_cards(game, sa, true, defined_param)
}

/// Core card resolution logic — shared by getTargetCards and getDefinedCardsOrTargeted.
/// Mirrors Java's private `SpellAbilityEffect.getCards(definedFirst, definedParam, sa)`.
fn get_cards(
    game: &GameState,
    sa: &SpellAbility,
    defined_first: bool,
    defined_param: &str,
) -> Vec<CardId> {
    let use_targets = sa.uses_targeting() && (!defined_first || !sa.params.has(defined_param));

    if use_targets {
        // Return targeted card(s)
        sa.target_chosen.target_card.into_iter().collect()
    } else {
        // Resolve Defined$ (or default to "Self")
        let defined = sa
            .params
            .get(defined_param)
            .unwrap_or("Self");

        // Handle " & "-separated definitions (e.g. "Self & Targeted")
        let mut result = Vec::new();
        for d in defined.split(" & ") {
            let d = d.trim();
            let cards = resolve_defined_cards_for_sa(game, sa, d);
            result.extend(cards);
        }
        result
    }
}

/// Get target players for a spell ability.
/// If the SA uses targeting, returns the chosen target player(s).
/// Otherwise, resolves the `Defined$` parameter (defaulting to "You").
///
/// Mirrors Java's `SpellAbilityEffect.getTargetPlayers(sa)`.
pub fn get_target_players(game: &GameState, sa: &SpellAbility) -> Vec<PlayerId> {
    get_players(game, sa, false, "Defined")
}

/// Get defined players, falling back to targeted players if no `Defined$` param.
///
/// Mirrors Java's `SpellAbilityEffect.getDefinedPlayersOrTargeted(sa)`.
pub fn get_defined_players_or_targeted(game: &GameState, sa: &SpellAbility) -> Vec<PlayerId> {
    get_players(game, sa, true, "Defined")
}

/// Core player resolution logic.
/// Mirrors Java's private `SpellAbilityEffect.getPlayers(definedFirst, definedParam, sa)`.
fn get_players(
    game: &GameState,
    sa: &SpellAbility,
    defined_first: bool,
    defined_param: &str,
) -> Vec<PlayerId> {
    let use_targets = sa.uses_targeting() && (!defined_first || !sa.params.has(defined_param));

    if use_targets {
        sa.target_chosen.target_player.into_iter().collect()
    } else {
        let defined = sa
            .params
            .get(defined_param)
            .unwrap_or("You");

        let mut result = Vec::new();
        for d in defined.split(" & ") {
            let d = d.trim();
            let players =
                ability_utils::resolve_defined_players(d, sa.activating_player, game);
            result.extend(players);
        }
        result
    }
}

/// Resolve a `Defined$` string to card IDs in the context of a spell ability.
/// Handles SA-specific defined values like "Targeted", "ParentTarget",
/// "TriggeredCard", etc., in addition to the base AbilityUtils definitions.
fn resolve_defined_cards_for_sa(
    game: &GameState,
    sa: &SpellAbility,
    defined: &str,
) -> Vec<CardId> {
    match defined {
        "Targeted" => sa.target_chosen.target_card.into_iter().collect(),
        "TriggeredCard" => sa.trigger_source.into_iter().collect(),
        _ => ability_utils::get_defined_cards(
            game,
            sa.source,
            defined,
            Some(sa.activating_player),
        ),
    }
}

/// Set up the "replace dying" replacement effect for cards that should
/// be exiled instead of dying this turn.
///
/// Mirrors Java's `SpellAbilityEffect.replaceDying(sa)`.
/// Currently a stub — the full replacement-effect registration requires
/// the replacement handler infrastructure. Effects that need this should
/// check `ReplaceDyingDefined$` / `ReplaceDyingValid$` params.
pub fn replace_dying(game: &GameState, sa: &SpellAbility) -> Vec<CardId> {
    if !sa.params.has("ReplaceDyingDefined") && !sa.params.has("ReplaceDyingValid") {
        return Vec::new();
    }

    // Check condition (currently only Kicked)
    if let Some(cond) = sa.params.get("ReplaceDyingCondition") {
        if cond == "Kicked" && !sa.kicked {
            return Vec::new();
        }
    }

    // Resolve which cards should be replaced
    if let Some(defined) = sa.params.get("ReplaceDyingDefined") {
        resolve_defined_cards_for_sa(game, sa, defined)
    } else {
        Vec::new()
    }
}
