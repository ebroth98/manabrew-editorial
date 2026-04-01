//! AbilityUtils — utility functions for ability resolution.
//!
//! Mirrors Java's `AbilityUtils.java`.
//! This is the single source of truth for helper functions used across effects.
//! The `effects::helpers` module re-exports everything from here for backward
//! compatibility.

use forge_foundation::{ColorSet, ZoneType};

use crate::card::filter_constants as fc;
use crate::card::{Card, CounterType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

fn parse_card_objects(sa: &SpellAbility, key: &str) -> Vec<CardId> {
    sa.trigger_objects
        .get(key)
        .into_iter()
        .flat_map(|value| value.split(','))
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .map(CardId)
        .collect()
}

fn parse_player_objects(sa: &SpellAbility, key: &str) -> Vec<PlayerId> {
    sa.trigger_objects
        .get(key)
        .into_iter()
        .flat_map(|value| value.split(','))
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .map(PlayerId)
        .collect()
}

fn push_unique_player(players: &mut Vec<PlayerId>, player: PlayerId) {
    if !players.contains(&player) {
        players.push(player);
    }
}

fn targeted_spell_abilities(sa: &SpellAbility, game: &GameState) -> Vec<SpellAbility> {
    let mut spells = Vec::new();
    if let Some(stack_id) = sa.target_chosen.target_stack_entry {
        if let Some(entry) = game.stack.find_by_id(stack_id) {
            unique_push_spell(&mut spells, entry.spell_ability.clone());
        }
    }
    spells
}

fn targeted_controller_players(sa: &SpellAbility, game: &GameState) -> Vec<PlayerId> {
    let mut players = Vec::new();
    if let Some(cid) = sa.target_chosen.target_card {
        push_unique_player(&mut players, game.card(cid).controller);
    }
    for spell in targeted_spell_abilities(sa, game) {
        push_unique_player(&mut players, spell.activating_player);
    }
    players
}

fn targeted_owner_players(sa: &SpellAbility, game: &GameState) -> Vec<PlayerId> {
    let mut players = Vec::new();
    if let Some(cid) = sa.target_chosen.target_card {
        push_unique_player(&mut players, game.card(cid).owner);
    }
    for spell in targeted_spell_abilities(sa, game) {
        if let Some(source) = spell.source {
            push_unique_player(&mut players, game.card(source).owner);
        }
    }
    players
}

fn unique_push_spell(spells: &mut Vec<SpellAbility>, spell: SpellAbility) {
    let spell_source = spell.source;
    let spell_api = spell.api;
    let spell_text = spell.ability_text.clone();
    let spell_target = spell.target_chosen.target_stack_entry;
    if spells.iter().any(|existing| {
        existing.source == spell_source
            && existing.api == spell_api
            && existing.ability_text == spell_text
            && existing.target_chosen.target_stack_entry == spell_target
    }) {
        return;
    }
    spells.push(spell);
}

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
    fn parse_player_object(sa: &SpellAbility, key: &str) -> Option<PlayerId> {
        parse_player_objects(sa, key).into_iter().next()
    }

    fn triggered_controller(sa: &SpellAbility, game: &GameState, key: &str) -> Option<PlayerId> {
        parse_player_object(sa, key).or_else(|| {
            parse_card_objects(sa, key)
                .into_iter()
                .next()
                .map(|cid| game.card(cid).controller)
        })
    }

    fn triggered_owner(sa: &SpellAbility, game: &GameState, key: &str) -> Option<PlayerId> {
        parse_card_objects(sa, key)
            .into_iter()
            .next()
            .map(|cid| game.card(cid).owner)
    }

    let key = defined.strip_prefix("Player.").unwrap_or(defined);
    if let Some(rest) = key.strip_prefix("Non") {
        return game.alive_players().into_iter().find(|pid| {
            !resolve_defined_players_with_sa(rest, sa, controller, game).contains(pid)
        });
    }
    match key {
        "TriggeredPlayer" | "TargetedPlayer" => sa
            .target_chosen
            .target_player
            .or_else(|| parse_player_object(sa, "Player")),
        "ThisTargetedPlayer" => sa.target_chosen.target_player,
        "TargetedOrController" => sa
            .target_chosen
            .target_player
            .or_else(|| targeted_controller_players(sa, game).into_iter().next()),
        "TriggeredTarget" | "TriggeredTargets" => {
            if let Some(player) = parse_player_object(sa, "TargetPlayer") {
                Some(player)
            } else if let Some(card) = parse_card_objects(sa, "TargetCard").into_iter().next() {
                Some(game.card(card).controller)
            } else {
                parse_player_object(sa, "Target").or_else(|| {
                    parse_card_objects(sa, "Target")
                        .into_iter()
                        .next()
                        .map(|cid| game.card(cid).controller)
                })
            }
        }
        "TriggeredTargetController" | "TriggeredTargetsController" => {
            if let Some(card) = parse_card_objects(sa, "TargetCard").into_iter().next() {
                Some(game.card(card).controller)
            } else if let Some(player) = parse_player_object(sa, "TargetPlayer") {
                Some(player)
            } else {
                parse_card_objects(sa, "Target")
                    .into_iter()
                    .next()
                    .map(|cid| game.card(cid).controller)
                    .or_else(|| parse_player_object(sa, "Target"))
            }
        }
        "TriggeredAttackedTarget" => parse_player_object(sa, "AttackedTarget"),
        "TriggeredAttackingPlayer" => parse_player_object(sa, "AttackingPlayer"),
        "TriggeredActivator" => parse_player_object(sa, "Activator"),
        "TriggeredOpponentVotedDiff" => parse_player_object(sa, "OpponentVotedDiff"),
        "TriggeredOpponentVotedSame" => parse_player_object(sa, "OpponentVotedSame"),
        "TriggeredCardController" => triggered_controller(sa, game, "Card"),
        "TriggeredCardOwner" => triggered_owner(sa, game, "Card"),
        "ReplacedCardController" => triggered_controller(sa, game, "ReplacedCard")
            .or_else(|| triggered_controller(sa, game, "Card")),
        "ReplacedCardOwner" => {
            triggered_owner(sa, game, "ReplacedCard").or_else(|| triggered_owner(sa, game, "Card"))
        }
        "TriggeredSourceController" => triggered_controller(sa, game, "Source"),
        "TriggeredPlayerController" => triggered_controller(sa, game, "Player"),
        "DefendingPlayer" | "TriggeredDefendingPlayer" => sa
            .target_chosen
            .target_player
            .or_else(|| Some(game.opponent_of(controller))),
        "TriggeredController" => sa
            .trigger_source
            .map(|cid| game.card(cid).controller)
            .or(sa.target_chosen.target_player),
        "TargetedController" | "ThisTargetedController" | "ParentTargetedController" => {
            targeted_controller_players(sa, game).into_iter().next()
        }
        "TargetedOwner" | "ThisTargetedOwner" => {
            targeted_owner_players(sa, game).into_iter().next()
        }
        _ => resolve_defined_player(key, controller, game),
    }
}

/// Resolve a Defined$ parameter to a list of player IDs with spell/trigger context.
pub fn resolve_defined_players_with_sa(
    defined: &str,
    sa: &SpellAbility,
    controller: PlayerId,
    game: &GameState,
) -> Vec<PlayerId> {
    let key = defined.strip_prefix("Player.").unwrap_or(defined);
    if key.contains(" & ") {
        let mut players = Vec::new();
        for part in key
            .split(" & ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            for pid in resolve_defined_players_with_sa(part, sa, controller, game) {
                push_unique_player(&mut players, pid);
            }
        }
        return players;
    }
    if let Some(rest) = key.strip_prefix("Non") {
        let excluded = resolve_defined_players_with_sa(rest, sa, controller, game);
        return game
            .alive_players()
            .into_iter()
            .filter(|pid| !excluded.contains(pid))
            .collect();
    }
    match key {
        "TriggeredPlayer" | "TargetedPlayer" => {
            let mut players = Vec::new();
            for player in sa
                .target_chosen
                .target_player
                .into_iter()
                .chain(parse_player_objects(sa, "Player"))
            {
                push_unique_player(&mut players, player);
            }
            players
        }
        "ThisTargetedPlayer" => sa.target_chosen.target_player.into_iter().collect(),
        "TargetedOrController" => {
            let mut players: Vec<_> = sa.target_chosen.target_player.into_iter().collect();
            for player in targeted_controller_players(sa, game) {
                push_unique_player(&mut players, player);
            }
            players
        }
        "TargetedController" | "ThisTargetedController" | "ParentTargetedController" => {
            targeted_controller_players(sa, game)
        }
        "TargetedOwner" | "ThisTargetedOwner" => targeted_owner_players(sa, game),
        "TriggeredTarget" | "TriggeredTargets" => {
            let mut players = Vec::new();
            let target_players = parse_player_objects(sa, "TargetPlayer");
            let target_cards = parse_card_objects(sa, "TargetCard");
            if !target_players.is_empty() {
                for player in target_players {
                    push_unique_player(&mut players, player);
                }
            } else if !target_cards.is_empty() {
                for cid in target_cards {
                    push_unique_player(&mut players, game.card(cid).controller);
                }
            } else {
                for player in parse_player_objects(sa, "Target") {
                    push_unique_player(&mut players, player);
                }
                for cid in parse_card_objects(sa, "Target") {
                    push_unique_player(&mut players, game.card(cid).controller);
                }
            }
            players
        }
        "TriggeredTargetController" | "TriggeredTargetsController" => {
            let mut players = Vec::new();
            let target_players = parse_player_objects(sa, "TargetPlayer");
            let target_cards = parse_card_objects(sa, "TargetCard");
            if !target_cards.is_empty() {
                for cid in target_cards {
                    push_unique_player(&mut players, game.card(cid).controller);
                }
            } else if !target_players.is_empty() {
                for player in target_players {
                    push_unique_player(&mut players, player);
                }
            } else {
                for player in parse_player_objects(sa, "Target") {
                    push_unique_player(&mut players, player);
                }
                for cid in parse_card_objects(sa, "Target") {
                    push_unique_player(&mut players, game.card(cid).controller);
                }
            }
            players
        }
        "TriggeredAttackedTarget" => parse_player_objects(sa, "AttackedTarget"),
        "TriggeredAttackedTargetAndYou" => {
            let mut players = parse_player_objects(sa, "AttackedTarget");
            push_unique_player(&mut players, controller);
            players
        }
        "TriggeredAttackingPlayer" => parse_player_objects(sa, "AttackingPlayer"),
        "TriggeredActivator" => parse_player_objects(sa, "Activator"),
        "TriggeredOpponentVotedDiff" => parse_player_objects(sa, "OpponentVotedDiff"),
        "TriggeredOpponentVotedSame" => parse_player_objects(sa, "OpponentVotedSame"),
        "TriggeredCardController" => parse_card_objects(sa, "Card")
            .into_iter()
            .map(|cid| game.card(cid).controller)
            .collect(),
        "TriggeredCardOwner" => parse_card_objects(sa, "Card")
            .into_iter()
            .map(|cid| game.card(cid).owner)
            .collect(),
        "ReplacedCardController" => {
            let replaced = parse_card_objects(sa, "ReplacedCard");
            let cards = if replaced.is_empty() {
                parse_card_objects(sa, "Card")
            } else {
                replaced
            };
            cards
                .into_iter()
                .map(|cid| game.card(cid).controller)
                .collect()
        }
        "ReplacedCardOwner" => {
            let replaced = parse_card_objects(sa, "ReplacedCard");
            let cards = if replaced.is_empty() {
                parse_card_objects(sa, "Card")
            } else {
                replaced
            };
            cards.into_iter().map(|cid| game.card(cid).owner).collect()
        }
        "TriggeredSourceController" => parse_card_objects(sa, "Source")
            .into_iter()
            .map(|cid| game.card(cid).controller)
            .collect(),
        "TriggeredPlayerController" => {
            let mut players = parse_player_objects(sa, "Player");
            for cid in parse_card_objects(sa, "Player") {
                push_unique_player(&mut players, game.card(cid).controller);
            }
            players
        }
        "DefendingPlayer" | "TriggeredDefendingPlayer" => {
            let mut players: Vec<_> = sa.target_chosen.target_player.into_iter().collect();
            let defending = game.opponent_of(controller);
            push_unique_player(&mut players, defending);
            players
        }
        _ => resolve_defined_players(key, controller, game),
    }
}

/// Resolve a Defined$ parameter to spell abilities with spell/trigger context.
/// Mirrors the Java `AbilityUtils.getDefinedSpellAbilities()` cases needed by
/// trigger and copy/counter effects.
pub fn resolve_defined_spell_abilities_with_sa(
    defined: &str,
    sa: &SpellAbility,
    game: &GameState,
) -> Vec<SpellAbility> {
    let key = defined.trim();
    let mut spells = Vec::new();

    match key {
        "TriggeredSpellAbility" => {
            for trigger_key in ["SpellAbility", "SourceSA", "Cause", "AbilityMana"] {
                if let Some(spell) = sa.get_triggering_spell_ability(trigger_key) {
                    unique_push_spell(&mut spells, spell.clone());
                }
            }
        }
        "SpellTargeted" => {
            if let Some(stack_id) = sa.target_chosen.target_stack_entry {
                if let Some(entry) = game.stack.find_by_id(stack_id) {
                    unique_push_spell(&mut spells, entry.spell_ability.clone());
                }
            }
        }
        "SourceSA" | "SpellAbility" | "AbilityMana" | "Cause" | "StackSa" => {
            if let Some(spell) = sa.get_triggering_spell_ability(key) {
                unique_push_spell(&mut spells, spell.clone());
            }
        }
        "TopStack" => {
            if let Some(spell) = game.stack.peek_ability() {
                unique_push_spell(&mut spells, spell.clone());
            }
        }
        _ => {}
    }

    spells
}

/// Resolve a Defined$ parameter to a list of player IDs.
/// Supports "You", "Opponent", "Each"/"All"/"Player" (all alive players).
/// Mirrors Java's AbilityUtils.getDefinedPlayers() for multi-player resolution.
pub fn resolve_defined_players(
    defined: &str,
    controller: PlayerId,
    game: &GameState,
) -> Vec<PlayerId> {
    if defined.contains(" & ") {
        let mut players = Vec::new();
        for part in defined
            .split(" & ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            for pid in resolve_defined_players(part, controller, game) {
                if !players.contains(&pid) {
                    players.push(pid);
                }
            }
        }
        return players;
    }
    if let Some(rest) = defined.strip_prefix("Non") {
        let excluded = resolve_defined_players(rest, controller, game);
        return game
            .alive_players()
            .into_iter()
            .filter(|pid| !excluded.contains(pid))
            .collect();
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    use crate::card::Card;
    use crate::ids::{CardId, PlayerId};

    fn make_card(
        game: &mut GameState,
        owner: PlayerId,
        controller: PlayerId,
        name: &str,
    ) -> CardId {
        let mut card = Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse("Creature"),
            ManaCost::parse("1"),
            ColorSet::COLORLESS,
            Some(1),
            Some(1),
            vec![],
            vec![],
        );
        card.controller = controller;
        game.create_card(card)
    }

    #[test]
    fn targeted_controller_ignores_player_targets() {
        let game = GameState::new(&["P0", "P1"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let mut sa = SpellAbility::new_simple(None, p0, "DB$ Discard");
        sa.target_chosen.target_player = Some(p1);

        assert_eq!(
            resolve_defined_player_with_sa("TargetedController", &sa, p0, &game),
            None
        );
        assert!(resolve_defined_players_with_sa("TargetedController", &sa, p0, &game).is_empty());
        assert_eq!(
            resolve_defined_players_with_sa("TargetedOrController", &sa, p0, &game),
            vec![p1]
        );
    }

    #[test]
    fn targeted_controller_and_owner_use_targeted_card_only() {
        let mut game = GameState::new(&["P0", "P1"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let target = make_card(&mut game, p0, p1, "Borrowed Bear");

        let mut sa = SpellAbility::new_simple(None, p0, "DB$ Draw");
        sa.target_chosen.target_card = Some(target);

        assert_eq!(
            resolve_defined_player_with_sa("TargetedController", &sa, p0, &game),
            Some(p1)
        );
        assert_eq!(
            resolve_defined_player_with_sa("TargetedOwner", &sa, p0, &game),
            Some(p0)
        );
        assert_eq!(
            resolve_defined_players_with_sa("TargetedController", &sa, p0, &game),
            vec![p1]
        );
        assert_eq!(
            resolve_defined_players_with_sa("TargetedOwner", &sa, p0, &game),
            vec![p0]
        );
    }

    #[test]
    fn this_targeted_player_stays_on_current_sa_targets() {
        let game = GameState::new(&["P0", "P1"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let mut sa = SpellAbility::new_simple(None, p0, "DB$ Token");
        sa.target_chosen.target_player = Some(p1);

        assert_eq!(
            resolve_defined_players_with_sa("ThisTargetedPlayer", &sa, p0, &game),
            vec![p1]
        );
    }
}

// ── Counter Type Parsing ─────────────────────────────────────────────

/// Parse a counter type string to CounterType enum (case-insensitive).
/// Unknown types produce `CounterType::Named(UPPER)` instead of silently
/// falling back to P1P1, so cards like Stocking the Pantry get the correct
/// SUPPLY counters.
pub fn parse_counter_type(s: &str) -> CounterType {
    crate::card::counter_type::parse_counter_type(s)
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
pub fn matches_valid_cards(card: &Card, filter: &str, activating_player: PlayerId) -> bool {
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

fn matches_valid_cards_single(card: &Card, filter: &str, activating_player: PlayerId) -> bool {
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
    card: &Card,
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
    card: &Card,
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
    game.player_record_discard(discard_player, 1);
    game.card_mut(card_id).set_discarded(true);

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
    trigger_handler.run_trigger(
        crate::event::TriggerType::DiscardedAll,
        crate::event::RunParams {
            card: Some(card_id),
            cards: Some(vec![card_id]),
            player: Some(discard_player),
            ..Default::default()
        },
        false,
    );
}

/// Remove the `MadnessExiled` marker from a card's granted keywords.
pub fn remove_madness_exiled_marker(card: &mut Card) {
    card.granted_keywords
        .retain(|k| k != crate::card::KEYWORD_MADNESS_EXILED);
}
