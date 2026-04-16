//! SpellAbilityEffect — base trait and utility free functions for effects.
//!
//! Mirrors Java's `SpellAbilityEffect.java`.
//! In Java this is an abstract class with many protected static helpers;
//! in Rust we keep the trait for interface parity and provide the utility
//! methods as free functions that take `(&GameState, &SpellAbility)`.

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::player::player_factory_util::add_replacement_effect;
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

    /// Tokenize a description string, replacing CARDNAME with the card's name.
    /// Mirrors Java's `SpellAbilityEffect.tokenizeString(SpellAbility, String)`.
    fn tokenize_string(&self, game: &GameState, sa: &SpellAbility, desc: &str) -> String {
        tokenize_string(game, sa, desc)
    }

    /// Add a "forget on moved" trigger for remembered cards.
    /// Mirrors Java's `SpellAbilityEffect.addForgetOnMovedTrigger(SpellAbility, Card, String)`.
    fn add_forget_on_moved_trigger(
        &self,
        game: &mut GameState,
        host_id: CardId,
        remembered_card_id: CardId,
    ) {
        add_forget_on_moved_trigger(game, host_id, remembered_card_id)
    }

    /// Create a temporary effect card in the command zone.
    /// Mirrors Java's `SpellAbilityEffect.createEffect(SpellAbility, Player, String, String)`.
    fn create_effect(
        &self,
        game: &mut GameState,
        sa: &SpellAbility,
        name: &str,
        image: &str,
    ) -> CardId {
        create_effect(game, sa, name, image)
    }

    /// Run the effect (resolve entry point).
    /// Mirrors Java's `SpellAbilityEffect.run(SpellAbility)`.
    fn run(&self, ctx: &mut super::effects::EffectContext, sa: &SpellAbility) {
        run(ctx, sa)
    }

    /// Track which card exiled another card, for "exile until" effects.
    /// Mirrors Java's `SpellAbilityEffect.handleExiledWith(SpellAbility, Card)`.
    fn handle_exiled_with(&self, game: &mut GameState, sa: &SpellAbility, exiled_card_id: CardId) {
        handle_exiled_with(game, sa, exiled_card_id)
    }

    /// Execute the exile-with command.
    /// Mirrors Java's `SpellAbilityEffect.exileEffectCommand(Game, SpellAbility, Card)`.
    fn exile_effect_command(
        &self,
        game: &mut GameState,
        trigger_handler: &mut crate::trigger::handler::TriggerHandler,
        sa: &SpellAbility,
        card_id: CardId,
    ) {
        exile_effect_command(game, trigger_handler, sa, card_id)
    }
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
        let defined = sa.params.get(defined_param).unwrap_or("Self");

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
    fn unique_push(players: &mut Vec<PlayerId>, player: PlayerId) {
        if !players.contains(&player) {
            players.push(player);
        }
    }

    fn ordered_players(game: &GameState, starter: PlayerId) -> Vec<PlayerId> {
        let Some(offset) = game.player_order.iter().position(|&pid| pid == starter) else {
            return game.player_order.clone();
        };

        (0..game.player_order.len())
            .map(|idx| game.player_order[(offset + idx) % game.player_order.len()])
            .collect()
    }

    fn sort_in_turn_order(game: &GameState, sa: &SpellAbility, players: &mut [PlayerId]) {
        let starter = sa
            .params
            .get("StartingWith")
            .and_then(|defined| {
                ability_utils::resolve_defined_players_with_sa(
                    defined,
                    sa,
                    sa.activating_player,
                    game,
                )
                .into_iter()
                .next()
            })
            .unwrap_or(game.turn.active_player);
        let ordered = ordered_players(game, starter);

        players.sort_by_key(|pid| {
            ordered
                .iter()
                .position(|ordered_pid| ordered_pid == pid)
                .unwrap_or(usize::MAX)
        });
    }

    let use_targets = sa.uses_targeting() && (!defined_first || !sa.params.has(defined_param));

    if use_targets {
        let mut result: Vec<_> = sa.target_chosen.target_player.into_iter().collect();
        sort_in_turn_order(game, sa, &mut result);
        result
    } else {
        let defined = sa.params.get(defined_param).unwrap_or("You");

        let mut result = Vec::new();
        for d in defined.split(" & ") {
            let d = d.trim();
            let players =
                ability_utils::resolve_defined_players_with_sa(d, sa, sa.activating_player, game);
            for player in players {
                unique_push(&mut result, player);
            }
        }
        sort_in_turn_order(game, sa, &mut result);
        result
    }
}

/// Resolve a `Defined$` string to card IDs in the context of a spell ability.
/// Handles SA-specific defined values like "Targeted", "ParentTarget",
/// "TriggeredCard", etc., in addition to the base AbilityUtils definitions.
fn resolve_defined_cards_for_sa(game: &GameState, sa: &SpellAbility, defined: &str) -> Vec<CardId> {
    fn parse_card_ids(csv: Option<&String>) -> Vec<CardId> {
        csv.into_iter()
            .flat_map(|value| value.split(','))
            .filter_map(|part| part.trim().parse::<u32>().ok())
            .map(CardId)
            .collect()
    }

    match defined {
        "Self" | "CARDNAME" => {
            if sa.is_trigger {
                if let (Some(source), Some(created_at)) =
                    (sa.trigger_source, sa.trigger_source_zone_timestamp)
                {
                    let current = game.card(source);
                    if current.zone_timestamp != created_at {
                        return Vec::new();
                    }
                }
            }
            sa.source.into_iter().collect()
        }
        "Targeted" => sa.target_chosen.target_card.into_iter().collect(),
        "TriggeredCard" | "TriggeredCardLKICopy" => {
            let cards = parse_card_ids(sa.trigger_objects.get("Card"));
            if cards.is_empty() {
                sa.trigger_source.into_iter().collect()
            } else {
                cards
            }
        }
        "ReplacedCard" => {
            let cards = parse_card_ids(sa.trigger_objects.get("ReplacedCard"));
            if cards.is_empty() {
                parse_card_ids(sa.trigger_objects.get("Card"))
            } else {
                cards
            }
        }
        "TriggeredNewCard" | "TriggeredNewCardLKICopy" => {
            let cards = parse_card_ids(sa.trigger_objects.get("NewCard"));
            if cards.is_empty() {
                sa.trigger_source.into_iter().collect()
            } else {
                cards
            }
        }
        "TriggeredAttackers" => parse_card_ids(sa.trigger_objects.get("Attackers")),
        "TriggeredAttacker" => parse_card_ids(sa.trigger_objects.get("Attacker")),
        "TriggeredBlocker" => parse_card_ids(sa.trigger_objects.get("Blocker")),
        "Explorer" => parse_card_ids(sa.trigger_objects.get("Explorer")),
        "Explored" => parse_card_ids(sa.trigger_objects.get("Explored")),
        _ => ability_utils::get_defined_cards(game, sa.source, defined, Some(sa.activating_player)),
    }
}

// ── SpellAbilityEffect utility functions ────────────────────────────

/// Tokenize a description string, replacing CARDNAME with the actual card name
/// and NICKNAME with a short version.
/// Mirrors Java's `SpellAbilityEffect.tokenizeString(SpellAbility, String)`.
pub fn tokenize_string(game: &GameState, sa: &SpellAbility, desc: &str) -> String {
    let card_name = sa
        .source
        .map(|cid| game.card(cid).card_name.clone())
        .unwrap_or_else(|| "CARDNAME".to_string());

    let mut result = desc.to_string();
    result = result.replace("CARDNAME", &card_name);
    result = result.replace("NICKNAME", &card_name);

    // Replace DAMAGE with the NumDmg parameter if present
    if let Some(num_dmg) = sa.params.get("NumDmg") {
        result = result.replace("DAMAGE", num_dmg);
    }

    // Replace AMOUNT with relevant numeric parameter
    if let Some(amount) = sa
        .params
        .get("Amount")
        .or_else(|| sa.params.get("NumCards"))
    {
        result = result.replace("AMOUNT", amount);
    }

    result
}

/// Add a "forget on moved" trigger to a card — when the card changes zones,
/// it is removed from its host's remembered list.
/// Mirrors Java's `SpellAbilityEffect.addForgetOnMovedTrigger(SpellAbility, Card, String)`.
pub fn add_forget_on_moved_trigger(
    game: &mut GameState,
    host_id: CardId,
    remembered_card_id: CardId,
) {
    // In the Rust engine, zone-change cleanup of remembered cards is handled
    // centrally in the zone-move logic. This function marks the card with a
    // flag so the zone-move system knows to clean up.
    // Store a SVar on the card so the zone-move system knows which host to clean up
    game.card_mut(remembered_card_id)
        .set_s_var("ForgetOnZoneChangeHost", &host_id.0.to_string());
}

/// Create a temporary "effect" card in the command zone.
/// Mirrors Java's `SpellAbilityEffect.createEffect(SpellAbility, Player, String, String)`.
///
/// Effect cards are invisible game objects that hold continuous effects,
/// delayed triggers, and other state that persists beyond a single resolution.
/// They are placed in the Command zone and cleaned up when their effect ends.
pub fn create_effect(game: &mut GameState, sa: &SpellAbility, name: &str, image: &str) -> CardId {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    let owner = sa.activating_player;
    let source_name = sa
        .source
        .map(|cid| game.card(cid).card_name.clone())
        .unwrap_or_default();

    let effect_name = if name.is_empty() {
        format!("{} Effect", source_name)
    } else {
        name.to_string()
    };

    let effect_card = crate::card::Card::new(
        CardId(0), // will be assigned by create_card
        effect_name,
        owner,
        CardTypeLine::parse("Effect"),
        ManaCost::parse(""),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    );

    let effect_id = game.create_card(effect_card);
    game.move_card(effect_id, forge_foundation::ZoneType::Command, owner);

    // Copy the image hint from the source card
    if let Some(source_id) = sa.source {
        let _ = image; // image parameter used in Java for UI; we track source_id instead
        game.card_mut(effect_id).effect_source = Some(source_id);
        let source_svars = game.card(source_id).svars.clone();
        game.card_mut(effect_id).set_svars_map(source_svars);
    }

    // Mark as an effect card (not a "real" card) via SVar
    game.card_mut(effect_id).set_s_var("IsEffectCard", "True");

    effect_id
}

/// Run/resolve a spell ability effect (the main entry point for effect dispatch).
/// Mirrors Java's `SpellAbilityEffect.run(SpellAbility)` which calls resolve().
///
/// In the Rust engine this delegates to the effect dispatch system.
pub fn run(ctx: &mut super::effects::EffectContext, sa: &SpellAbility) {
    super::effects::resolve_effect(ctx, sa);
}

/// Track which card exiled another card, for "exile until" effects.
/// Mirrors Java's `SpellAbilityEffect.handleExiledWith(SpellAbility, Card)`.
///
/// Sets the `exiled_with` field on the exiled card and adds it to the
/// source card's imprinted list.
pub fn handle_exiled_with(game: &mut GameState, sa: &SpellAbility, exiled_card_id: CardId) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    game.card_mut(exiled_card_id).set_exiled_by(Some(source_id));
    game.card_mut(source_id).add_imprinted_card(exiled_card_id);
}

/// Execute the "exile with" command — exile a card and track the exile source.
/// Mirrors Java's `SpellAbilityEffect.exileEffectCommand(Game, SpellAbility, Card)`.
///
/// Moves the card to exile, sets up the exiled_with tracking, and optionally
/// adds it to the source's remembered list.
pub fn exile_effect_command(
    game: &mut GameState,
    trigger_handler: &mut crate::trigger::handler::TriggerHandler,
    sa: &SpellAbility,
    card_id: CardId,
) {
    let owner = game.card(card_id).owner;
    let old_zone = game.card(card_id).zone;

    // Move the card to exile
    game.move_card(card_id, forge_foundation::ZoneType::Exile, owner);

    // Register zone triggers
    trigger_handler.register_active_trigger(game, card_id);
    super::effects::zone_triggers::emit_zone_trigger(
        trigger_handler,
        card_id,
        old_zone,
        forge_foundation::ZoneType::Exile,
    );

    // Set up the exiled_with relationship
    handle_exiled_with(game, sa, card_id);

    // Remember the exiled card if requested
    if sa.params.has("RememberExiled") {
        if let Some(source_id) = sa.source {
            game.card_mut(source_id).add_remembered_card(card_id);
        }
    }
}

/// Set up the "replace dying" replacement effect for cards that should
/// be exiled instead of dying this turn.
///
/// Mirrors Java's `SpellAbilityEffect.replaceDying(sa)`.
pub fn replace_dying(game: &mut GameState, sa: &SpellAbility) -> Vec<CardId> {
    if !sa.params.has("ReplaceDyingDefined") && !sa.params.has("ReplaceDyingValid") {
        return Vec::new();
    }

    // Check condition (currently only Kicked)
    if let Some(cond) = sa.params.get("ReplaceDyingCondition") {
        if cond == "Kicked" && !sa.kicked {
            return Vec::new();
        }
    }

    let cards = if let Some(defined) = sa.params.get("ReplaceDyingDefined") {
        let cards = resolve_defined_cards_for_sa(game, sa, defined);
        if cards.is_empty() {
            return Vec::new();
        }
        cards
    } else {
        Vec::new()
    };

    let effect_name = sa
        .source
        .map(|source_id| format!("{}'s Effect", game.card(source_id).card_name))
        .unwrap_or_else(|| "Effect".to_string());
    let effect_id = create_effect(game, sa, &effect_name, "");
    {
        let effect = game.card_mut(effect_id);
        effect.add_remembered_cards(cards.iter().copied());
        effect.set_forget_on_moved_origin(Some(forge_foundation::ZoneType::Battlefield));
        effect.set_exile_when_no_remembered(true);
        effect.set_temp_effect_until_eot(true);
    }

    let valid = sa.params.get("ReplaceDyingValid").unwrap_or("Card.IsRemembered");
    let zone = sa.params.get("ReplaceDyingZone").unwrap_or("Exile");
    let mut replacement_raw = format!(
        "R$ Event$ Moved | ValidLKI$ {} | Origin$ Battlefield | Destination$ Graveyard | NewDestination$ {} | Description$ If that permanent would die this turn, exile it instead.",
        valid, zone
    );
    if sa.params.has("ReplaceDyingExiledWith") {
        replacement_raw.push_str(" | ExiledWithEffectSource$ True");
    }

    let effect = game.card_mut(effect_id);
    add_replacement_effect(effect, &replacement_raw);

    cards
}
