//! Effect resolution system.
//!
//! Each effect type lives in its own file, mirroring the Java Forge
//! `ability/effects/` package (204 files). Effects are dispatched by
//! API type string extracted from the ability text.

pub mod add_phase_effect;
pub mod add_turn_effect;
pub mod animate_all_effect;
pub mod animate_effect;
pub mod attach_effect;
pub mod balance_effect;
pub mod become_monarch_effect;
pub mod charm_effect;
pub mod choose_card_effect;
pub mod choose_color_effect;
pub mod cleanup_effect;
pub mod clone_effect;
pub mod change_zone_all_effect;
pub mod change_zone_effect;
pub mod control_gain_effect;
pub mod control_gain_variant_effect;
pub mod copy_permanent_effect;
pub mod counter_effect;
pub mod counters_put_effect;
pub mod counters_remove_effect;
pub mod damage_all_effect;
pub mod damage_deal_effect;
pub mod destroy_all_effect;
pub mod destroy_effect;
pub mod dig_effect;
pub mod dig_multiple_effect;
pub mod discard_effect;
pub mod draw_effect;
pub mod end_combat_phase_effect;
pub mod end_turn_effect;
pub mod fight_effect;
pub mod fog_effect;
pub mod game_draw_effect;
pub mod game_loss_effect;
pub mod game_win_effect;
pub mod life_exchange_effect;
pub mod life_gain_effect;
pub mod life_lose_effect;
pub mod life_set_effect;
pub mod look_at_effect;
pub mod mana_effect;
pub mod mill_effect;
pub mod phases_effect;
pub mod peek_and_reveal_effect;
pub mod play_effect;
pub mod poison_effect;
pub mod repeat_each_effect;
pub mod power_exchange_effect;
pub mod pump_all_effect;
pub mod pump_effect;
pub mod rearrange_top_of_library_effect;
pub mod regenerate_effect;
pub mod reveal_effect;
pub mod reveal_hand_effect;
pub mod reverse_turn_order_effect;
pub mod sacrifice_all_effect;
pub mod sacrifice_effect;
pub mod scry_effect;
pub mod set_state_effect;
pub mod skip_phase_effect;
pub mod skip_turn_effect;
pub mod surveil_effect;
pub mod take_initiative_effect;
pub mod tap_all_effect;
pub mod tap_effect;
pub mod token_effect;
pub mod untap_all_effect;
pub mod untap_effect;

use std::collections::HashMap;

use forge_foundation::{ColorSet, ZoneType};

use crate::agent::PlayerAgent;
use crate::card::{CardInstance, CounterType};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::spellability::SpellAbility;
use crate::trigger::handler::TriggerHandler;

/// Everything an effect needs to resolve.
pub struct EffectContext<'a> {
    pub game: &'a mut GameState,
    pub agents: &'a mut [Box<dyn PlayerAgent>],
    pub trigger_handler: &'a mut TriggerHandler,
    pub token_templates: &'a HashMap<String, CardInstance>,
    pub mana_pools: &'a mut Vec<ManaPool>,
    /// CardId of the parent SA's chosen target card, propagated through the
    /// sub-ability chain so that `Defined$ ParentTarget` effects can resolve it.
    /// Mirrors Java's `SpellAbility.getParentTargetCard()` (via getRootAbility()).
    pub parent_target_card: Option<CardId>,
}

/// Check if a conditional gate on this SA is satisfied.
/// Handles `Condition$ Kicked` (simple gate) and `ConditionCheckSVar$ Kicked` (SVar-based gate).
/// Mirrors Java's `SpellAbility.checkConditions()`.
fn check_condition(sa: &SpellAbility) -> bool {
    // Check Condition$ Kicked (most common pattern: simple kicked gate)
    if let Some(cond) = sa.params.get("Condition") {
        if cond == "Kicked" {
            return sa.kicked;
        }
    }
    // Check ConditionCheckSVar$ Kicked (SVar-based kicked gate)
    if let Some(cond) = sa.params.get("ConditionCheckSVar") {
        if cond == "Kicked" || cond == "X:Kicked" {
            return sa.kicked;
        }
    }
    true
}

/// Resolve a single SpellAbility node's effect by dispatching on its API type.
/// Mirrors Java's `AbilityUtils.resolveApiAbility(sa)`.
pub fn resolve_effect(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Check condition gate (e.g. Kicked) — skip this effect if condition not met
    if !check_condition(sa) {
        return;
    }

    // Handle Repeat$ — repeat the effect N times (for Multikicker/Replicate-like scaling).
    // Mirrors Java's AbilityUtils.handleRepeatParam().
    let repeat_count = if let Some(repeat_val) = sa.params.get("Repeat") {
        match repeat_val.as_str() {
            "KickerNum" => sa.kick_count as i32,
            _ => 1,
        }
    } else {
        1
    };

    for _ in 0..repeat_count {
        resolve_effect_once(ctx, sa);
    }
}

/// Inner dispatch for a single execution of an effect.
fn resolve_effect_once(ctx: &mut EffectContext, sa: &SpellAbility) {
    let api_type = sa.api.as_deref().unwrap_or_else(|| {
        // Fallback: detect from ability text (for backwards compat)
        detect_api_type_from_text(&sa.ability_text)
    });

    match api_type {
        "DealDamage" => damage_deal_effect::resolve(ctx, sa),
        "GainLife" => life_gain_effect::resolve(ctx, sa),
        "LoseLife" => life_lose_effect::resolve(ctx, sa),
        "PutCounter" => counters_put_effect::resolve(ctx, sa),
        "RemoveCounter" => counters_remove_effect::resolve(ctx, sa),
        "Poison" => poison_effect::resolve(ctx, sa),
        "Pump" => pump_effect::resolve(ctx, sa),
        "Destroy" => destroy_effect::resolve(ctx, sa),
        "Draw" => draw_effect::resolve(ctx, sa),
        "ChangeZoneAll" => change_zone_all_effect::resolve(ctx, sa),
        "ChangeZone" => change_zone_effect::resolve(ctx, sa),
        "SacrificeAll" => sacrifice_all_effect::resolve(ctx, sa),
        "Sacrifice" => sacrifice_effect::resolve(ctx, sa),
        "CopyPermanent" => copy_permanent_effect::resolve(ctx, sa),
        "Token" => token_effect::resolve(ctx, sa),
        "Mana" => mana_effect::resolve(ctx, sa),
        // Library manipulation (issue #15)
        "Mill" => mill_effect::resolve(ctx, sa),
        "Scry" => scry_effect::resolve(ctx, sa),
        "Surveil" => surveil_effect::resolve(ctx, sa),
        "Dig" => dig_effect::resolve(ctx, sa),
        "DigMultiple" => dig_multiple_effect::resolve(ctx, sa),
        "RearrangeTopOfLibrary" => rearrange_top_of_library_effect::resolve(ctx, sa),
        // Reveal / Look (informational)
        "Reveal" => reveal_effect::resolve(ctx, sa),
        "RevealHand" => reveal_hand_effect::resolve(ctx, sa),
        "LookAt" => look_at_effect::resolve(ctx, sa),
        // Modal effects (issue #18)
        "Charm" => charm_effect::resolve(ctx, sa),
        // Double-faced card / transform effects (issue #19)
        "PeekAndReveal" => peek_and_reveal_effect::resolve(ctx, sa),
        "SetState" => set_state_effect::resolve(ctx, sa),
        "Cleanup" => cleanup_effect::resolve(ctx, sa),
        // Counter, Control, Fight, Discard, Attach (issue #16)
        "Counter" => counter_effect::resolve(ctx, sa),
        "ControlGain" => control_gain_effect::resolve(ctx, sa),
        "Fight" => fight_effect::resolve(ctx, sa),
        "Discard" => discard_effect::resolve(ctx, sa),
        "Attach" => attach_effect::resolve(ctx, sa),
        // Mass / board-wide effects (issue #17)
        "DestroyAll" => destroy_all_effect::resolve(ctx, sa),
        "DamageAll" => damage_all_effect::resolve(ctx, sa),
        "PumpAll" => pump_all_effect::resolve(ctx, sa),
        "TapAll" => tap_all_effect::resolve(ctx, sa),
        "UntapAll" => untap_all_effect::resolve(ctx, sa),
        // Player & game-state effects (issue #22)
        "Tap" => tap_effect::resolve(ctx, sa),
        "Untap" => untap_effect::resolve(ctx, sa),
        "LifeSet" => life_set_effect::resolve(ctx, sa),
        "LifeExchange" => life_exchange_effect::resolve(ctx, sa),
        "GameWin" => game_win_effect::resolve(ctx, sa),
        "GameLoss" => game_loss_effect::resolve(ctx, sa),
        "GameDraw" => game_draw_effect::resolve(ctx, sa),
        "AddTurn" => add_turn_effect::resolve(ctx, sa),
        "Fog" => fog_effect::resolve(ctx, sa),
        // New player & game-state effects (issue #22, expanded)
        "ReverseTurnOrder" => reverse_turn_order_effect::resolve(ctx, sa),
        "EndCombatPhase" => end_combat_phase_effect::resolve(ctx, sa),
        "EndTurn" => end_turn_effect::resolve(ctx, sa),
        "PowerExchange" => power_exchange_effect::resolve(ctx, sa),
        "BecomeMonarch" => become_monarch_effect::resolve(ctx, sa),
        "TakeInitiative" => take_initiative_effect::resolve(ctx, sa),
        "SkipTurn" => skip_turn_effect::resolve(ctx, sa),
        "SkipPhase" => skip_phase_effect::resolve(ctx, sa),
        "AddPhase" => add_phase_effect::resolve(ctx, sa),
        "Phases" => phases_effect::resolve(ctx, sa),
        "Regenerate" => regenerate_effect::resolve(ctx, sa),
        // Cast from exile / without mana cost (Rebound, etc.) (issue #21)
        "Play" => play_effect::resolve(ctx, sa),
        // Critical effects (issue #52)
        "Animate" => animate_effect::resolve(ctx, sa),
        "AnimateAll" => animate_all_effect::resolve(ctx, sa),
        "Balance" => balance_effect::resolve(ctx, sa),
        "ChooseCard" => choose_card_effect::resolve(ctx, sa),
        "ChooseColor" => choose_color_effect::resolve(ctx, sa),
        "Clone" => clone_effect::resolve(ctx, sa),
        "ControlGainVariant" => control_gain_variant_effect::resolve(ctx, sa),
        "RepeatEach" => repeat_each_effect::resolve(ctx, sa),
        _ => {} // Unimplemented effect — silently skip
    }
}

/// Fallback: detect API type from raw ability text via contains-matching.
/// Only used when `SpellAbility.api` is None (shouldn't happen for properly
/// parsed abilities, but kept for backward compatibility).
fn detect_api_type_from_text(ability: &str) -> &'static str {
    // Order matters — check longer names first
    // ChangeZoneAll must be checked before ChangeZone, SacrificeAll before Sacrifice
    // RevealHand before Reveal, DigMultiple before Dig
    if ability.contains("DealDamage") {
        "DealDamage"
    } else if ability.contains("GainLife") {
        "GainLife"
    } else if ability.contains("LoseLife") {
        "LoseLife"
    } else if ability.contains("PutCounter") {
        "PutCounter"
    } else if ability.contains("$ Pump") {
        "Pump"
    } else if ability.contains("CopyPermanent") {
        "CopyPermanent"
    } else if ability.contains("Destroy") {
        "Destroy"
    } else if ability.contains("Draw") {
        "Draw"
    } else if ability.contains("ChangeZoneAll") {
        "ChangeZoneAll"
    } else if ability.contains("ChangeZone") {
        "ChangeZone"
    } else if ability.contains("SacrificeAll") {
        "SacrificeAll"
    } else if ability.contains("Sacrifice") {
        "Sacrifice"
    } else if ability.contains("Token") {
        "Token"
    } else if ability.contains("Mana") {
        "Mana"
    } else if ability.contains("Mill") {
        "Mill"
    } else if ability.contains("Scry") {
        "Scry"
    } else if ability.contains("Surveil") {
        "Surveil"
    } else if ability.contains("DigMultiple") {
        "DigMultiple"
    } else if ability.contains("$ Dig") {
        "Dig"
    } else if ability.contains("RearrangeTopOfLibrary") {
        "RearrangeTopOfLibrary"
    } else if ability.contains("RevealHand") {
        "RevealHand"
    } else if ability.contains("Reveal") {
        "Reveal"
    } else if ability.contains("LookAt") {
        "LookAt"
    } else if ability.contains("$ Charm") {
        "Charm"
    } else if ability.contains("PeekAndReveal") {
        "PeekAndReveal"
    } else if ability.contains("$ SetState") {
        "SetState"
    } else if ability.contains("$ Cleanup") {
        "Cleanup"
    } else if ability.contains("RemoveCounter") {
        "RemoveCounter"
    } else if ability.contains("$ Poison") {
        "Poison"
    } else if ability.contains("$ Counter") {
        "Counter"
    } else if ability.contains("ControlGain") {
        "ControlGain"
    } else if ability.contains("$ Fight") {
        "Fight"
    } else if ability.contains("$ Discard") {
        "Discard"
    } else if ability.contains("$ Attach") {
        "Attach"
    // Mass / board-wide effects (issue #17) — longer names first
    } else if ability.contains("DestroyAll") {
        "DestroyAll"
    } else if ability.contains("DamageAll") {
        "DamageAll"
    } else if ability.contains("PumpAll") {
        "PumpAll"
    } else if ability.contains("TapAll") {
        "TapAll"
    } else if ability.contains("UntapAll") {
        "UntapAll"
    // Player & game-state effects (issue #22) — check longer names first
    } else if ability.contains("LifeExchange") {
        "LifeExchange"
    } else if ability.contains("LifeSet") {
        "LifeSet"
    } else if ability.contains("GameWin") {
        "GameWin"
    } else if ability.contains("GameLoss") {
        "GameLoss"
    } else if ability.contains("GameDraw") {
        "GameDraw"
    } else if ability.contains("AddTurn") {
        "AddTurn"
    } else if ability.contains("$ Fog") {
        "Fog"
    } else if ability.contains("$ Tap") {
        "Tap"
    } else if ability.contains("$ Untap") {
        "Untap"
    // New player & game-state effects (issue #22, expanded)
    } else if ability.contains("ReverseTurnOrder") {
        "ReverseTurnOrder"
    } else if ability.contains("EndCombatPhase") {
        "EndCombatPhase"
    } else if ability.contains("EndTurn") {
        "EndTurn"
    } else if ability.contains("PowerExchange") {
        "PowerExchange"
    } else if ability.contains("BecomeMonarch") {
        "BecomeMonarch"
    } else if ability.contains("TakeInitiative") {
        "TakeInitiative"
    } else if ability.contains("SkipTurn") {
        "SkipTurn"
    } else if ability.contains("SkipPhase") {
        "SkipPhase"
    } else if ability.contains("AddPhase") {
        "AddPhase"
    } else if ability.contains("$ Phases") {
        "Phases"
    } else if ability.contains("$ Regenerate") {
        "Regenerate"
    // Critical effects (issue #52) — AnimateAll before Animate
    } else if ability.contains("AnimateAll") {
        "AnimateAll"
    } else if ability.contains("$ Animate") {
        "Animate"
    } else if ability.contains("$ Balance") {
        "Balance"
    } else if ability.contains("ChooseCard") {
        "ChooseCard"
    } else if ability.contains("ChooseColor") {
        "ChooseColor"
    } else if ability.contains("$ Clone") {
        "Clone"
    } else if ability.contains("ControlGainVariant") {
        "ControlGainVariant"
    } else if ability.contains("RepeatEach") {
        "RepeatEach"
    } else {
        ""
    }
}

// ── Shared helpers used by multiple effects ───────────────────────────

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

/// Resolve a numeric parameter that may be either a literal integer or an SVar
/// reference (like `X`). Handles `Count$Kicked.A.B` SVars: returns `A` if kicked,
/// `B` otherwise.
/// Mirrors Java's `AbilityUtils.calculateAmount(sa, paramName, sa)`.
pub fn resolve_numeric_svar(
    game: &GameState,
    sa: &SpellAbility,
    param_name: &str,
    default: i32,
) -> i32 {
    let val_str = match sa.params.get(param_name) {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return default,
    };

    // Try direct integer parse first
    if let Ok(n) = val_str.trim().parse::<i32>() {
        return n;
    }
    // Try with leading + sign (e.g. "+3")
    if let Some(stripped) = val_str.trim().strip_prefix('+') {
        if let Ok(n) = stripped.parse::<i32>() {
            return n;
        }
    }

    // It's an SVar reference — look it up on the source card
    if let Some(source_id) = sa.source {
        if let Some(svar_expr) = game.card(source_id).svars.get(val_str.trim()) {
            return evaluate_svar(svar_expr, sa);
        }
    }

    default
}

/// Evaluate a simple SVar expression.
/// Supports `Count$Kicked.A.B` (returns A if kicked, B otherwise)
/// and `Count$KickedCount` (returns the multikicker count).
pub fn evaluate_svar(expr: &str, sa: &SpellAbility) -> i32 {
    // Count$KickedCount — return the multikicker count (for Multikicker effects)
    if expr == "Count$KickedCount" {
        return sa.kick_count as i32;
    }
    // Count$Kicked.X.Y — if kicked return X, else return Y
    if let Some(rest) = expr.strip_prefix("Count$Kicked.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let kicked_val = parts[0].parse::<i32>().unwrap_or(0);
            let normal_val = parts[1].parse::<i32>().unwrap_or(0);
            return if sa.kicked { kicked_val } else { normal_val };
        }
    }
    // Fallback: try parsing as integer
    expr.parse::<i32>().unwrap_or(0)
}

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
        _ => None,
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

/// Parse a counter type string to CounterType enum (case-insensitive).
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
        _ => CounterType::P1P1, // fallback
    }
}

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

/// Convert a Produced$ value (e.g. "G", "R", "W") to a ManaAtom.
/// Re-exported from the mana module for convenience in effect files.
pub use crate::mana::mana_atom_from_produced;

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
pub fn matches_valid_cards(
    card: &CardInstance,
    filter: &str,
    activating_player: PlayerId,
) -> bool {
    if filter.is_empty() || filter == "Card" {
        return true;
    }

    let parts: Vec<&str> = filter.split('.').collect();
    let type_part = parts[0];

    // ── Type check ──────────────────────────────────────────────────────────
    let type_matches = match type_part {
        "Creature" => card.is_creature(),
        "Land" => card.is_land(),
        "Artifact" => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case("Artifact")),
        "Enchantment" => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case("Enchantment")),
        "Planeswalker" => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case("Planeswalker")),
        "Instant" => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case("Instant")),
        "Sorcery" => card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case("Sorcery")),
        "Permanent" | "Card" => true,
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
        "YouCtrl" => card.controller == activating_player,
        "OppCtrl" => card.controller != activating_player,
        "Basic" => card.type_line.is_basic(),
        "kicked" => card.kicked,
        "withFlying" => {
            card.keywords.iter().any(|k| k.eq_ignore_ascii_case("Flying"))
                || card
                    .granted_keywords
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case("Flying"))
        }
        _ => {
            // Color filters: "nonBlack", "nonRed", "nonWhite", etc.
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

    let parts: Vec<&str> = change_type.split('.').collect();
    let type_part = parts[0];

    let type_matches = match type_part {
        "Land" => card.is_land(),
        "Creature" => card.is_creature(),
        "Card" => true,
        _ => true,
    };

    if !type_matches {
        return false;
    }

    for &qualifier in &parts[1..] {
        match qualifier {
            "Basic" => {
                if !card.type_line.is_basic() {
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

/// Emit a ChangesZone trigger event. Used by multiple zone-moving effects.
pub fn emit_zone_trigger(
    trigger_handler: &mut TriggerHandler,
    card_id: CardId,
    origin: ZoneType,
    destination: ZoneType,
) {
    trigger_handler.run_trigger(
        TriggerType::ChangesZone,
        RunParams {
            card: Some(card_id),
            origin: Some(origin),
            destination: Some(destination),
            ..Default::default()
        },
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_num_dmg_test() {
        assert_eq!(
            parse_num_dmg("SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ test"),
            3
        );
    }

    #[test]
    fn parse_param_test() {
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumAtt$ "),
            Some(3)
        );
        assert_eq!(
            parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumDef$ "),
            Some(3)
        );
        assert_eq!(parse_param("SP$ Draw | NumCards$ 2", "NumCards$ "), Some(2));
    }

    #[test]
    fn detect_api_type_fallback() {
        assert_eq!(
            detect_api_type_from_text("something with ChangeZoneAll"),
            "ChangeZoneAll"
        );
        assert_eq!(
            detect_api_type_from_text("something with ChangeZone"),
            "ChangeZone"
        );
        assert_eq!(
            detect_api_type_from_text("something with SacrificeAll"),
            "SacrificeAll"
        );
        assert_eq!(
            detect_api_type_from_text("something with Sacrifice"),
            "Sacrifice"
        );
    }
}
