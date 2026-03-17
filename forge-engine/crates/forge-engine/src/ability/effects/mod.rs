//! Effect resolution system.
//!
//! Each effect type lives in its own file, mirroring the Java Forge
//! `ability/effects/` package (204 files). Effects are dispatched by
//! API type string extracted from the ability text.

pub mod activate_ability_effect;
pub mod add_phase_effect;
pub mod add_turn_effect;
pub mod animate_all_effect;
pub mod animate_effect;
pub mod attach_effect;
pub mod balance_effect;
pub mod become_monarch_effect;
pub mod branch_effect;
pub mod change_zone_all_effect;
pub mod change_zone_effect;
pub mod charm_effect;
pub mod choose_card_effect;
pub mod choose_color_effect;
pub mod choose_direction_effect;
pub mod choose_even_odd_effect;
pub mod choose_number_effect;
pub mod choose_player_effect;
pub mod choose_source_effect;
pub mod choose_type_effect;
pub mod cleanup_effect;
pub mod clone_effect;
pub mod connive_effect;
pub mod control_gain_effect;
pub mod control_gain_variant_effect;
pub mod copy_permanent_effect;
pub mod copy_spell_ability_effect;
pub mod counter_effect;
pub mod counters_put_all_effect;
pub mod counters_put_effect;
pub mod counters_put_or_remove_effect;
pub mod counters_remove_effect;
pub mod damage_all_effect;
pub mod damage_deal_effect;
pub mod delayed_trigger_effect;
pub mod destroy_all_effect;
pub mod destroy_effect;
pub mod detain_effect;
pub mod dig_effect;
pub mod dig_multiple_effect;
pub mod dig_until_effect;
pub mod discard_effect;
pub mod drain_mana_effect;
pub mod draw_effect;
pub mod each_damage_effect;
pub mod effect_effect;
pub mod encode_effect;
pub mod end_combat_phase_effect;
pub mod end_turn_effect;
pub mod explore_effect;
pub mod fight_effect;
pub mod flip_a_coin_effect;
pub mod fog_effect;
pub mod game_draw_effect;
pub mod game_loss_effect;
pub mod game_win_effect;
pub mod goad_effect;
pub mod life_exchange_effect;
pub mod life_gain_effect;
pub mod life_lose_effect;
pub mod life_set_effect;
pub mod look_at_effect;
pub mod mana_effect;
pub mod mana_reflected_effect;
pub mod mill_effect;
pub mod move_counter_effect;
pub mod must_block_effect;
pub mod name_card_effect;
pub mod peek_and_reveal_effect;
pub mod phases_effect;
pub mod play_effect;
pub mod poison_effect;
pub mod power_exchange_effect;
pub mod prevent_damage_effect;
pub mod proliferate_effect;
pub mod protection_all_effect;
pub mod protection_effect;
pub mod pump_all_effect;
pub mod pump_effect;
pub mod rearrange_top_of_library_effect;
pub mod regenerate_effect;
pub mod remove_from_combat_effect;
pub mod repeat_each_effect;
pub mod reveal_effect;
pub mod reveal_hand_effect;
pub mod reverse_turn_order_effect;
pub mod roll_dice_effect;
pub mod sacrifice_all_effect;
pub mod sacrifice_effect;
pub mod scry_effect;
pub mod set_state_effect;
pub mod shuffle_effect;
pub mod skip_phase_effect;
pub mod skip_turn_effect;
pub mod surveil_effect;
pub mod take_initiative_effect;
pub mod tap_all_effect;
pub mod tap_effect;
pub mod tap_or_untap_all_effect;
pub mod tap_or_untap_effect;
pub mod time_travel_effect;
pub mod token_effect;
pub mod two_piles_effect;
pub mod untap_all_effect;
pub mod untap_effect;

use std::collections::HashMap;

use forge_foundation::{ColorSet, ZoneType};

use crate::agent::PlayerAgent;
use crate::card::{CardInstance, CounterType};
use crate::cost::{parse_cost, Cost, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::spellability::SpellAbility;
use crate::trigger::handler::TriggerHandler;

/// Generates both `IMPLEMENTED_API_TYPES` and `resolve_effect_once` from a
/// single source of truth. Adding a new effect requires only one entry.
macro_rules! effect_dispatch {
    ( $( $api:literal => $handler:path ),* $(,)? ) => {
        /// All API type strings that have implemented effect handlers.
        /// Used by the fuzz card pool filter to exclude cards with unimplemented effects.
        pub const IMPLEMENTED_API_TYPES: &[&str] = &[ $( $api ),* ];

        /// Inner dispatch for a single execution of an effect.
        fn resolve_effect_once(ctx: &mut EffectContext, sa: &SpellAbility) {
            let api_type = sa.api.as_deref().unwrap_or_else(|| {
                // Fallback: detect from ability text (for backwards compat)
                detect_api_type_from_text(&sa.ability_text)
            });
            match api_type {
                $( $api => $handler(ctx, sa), )*
                _ => {} // Unimplemented effect — silently skip
            }
        }
    };
}

effect_dispatch! {
    "DealDamage" => damage_deal_effect::resolve,
    "Branch" => branch_effect::resolve,
    "GainLife" => life_gain_effect::resolve,
    "LoseLife" => life_lose_effect::resolve,
    "PutCounter" => counters_put_effect::resolve,
    "RemoveCounter" => counters_remove_effect::resolve,
    "Poison" => poison_effect::resolve,
    "Pump" => pump_effect::resolve,
    "Destroy" => destroy_effect::resolve,
    "Draw" => draw_effect::resolve,
    "ChangeZoneAll" => change_zone_all_effect::resolve,
    "ChangeZone" => change_zone_effect::resolve,
    "SacrificeAll" => sacrifice_all_effect::resolve,
    "Sacrifice" => sacrifice_effect::resolve,
    "CopyPermanent" => copy_permanent_effect::resolve,
    "Token" => token_effect::resolve,
    "Mana" => mana_effect::resolve,
    "ManaReflected" => mana_reflected_effect::resolve,
    "Mill" => mill_effect::resolve,
    "Scry" => scry_effect::resolve,
    "Surveil" => surveil_effect::resolve,
    "Dig" => dig_effect::resolve,
    "DigMultiple" => dig_multiple_effect::resolve,
    "RearrangeTopOfLibrary" => rearrange_top_of_library_effect::resolve,
    "Reveal" => reveal_effect::resolve,
    "RevealHand" => reveal_hand_effect::resolve,
    "LookAt" => look_at_effect::resolve,
    "Charm" => charm_effect::resolve,
    "PeekAndReveal" => peek_and_reveal_effect::resolve,
    "SetState" => set_state_effect::resolve,
    "Cleanup" => cleanup_effect::resolve,
    "Counter" => counter_effect::resolve,
    "ControlGain" => control_gain_effect::resolve,
    "GainControl" => control_gain_effect::resolve,
    "Fight" => fight_effect::resolve,
    "Discard" => discard_effect::resolve,
    "Attach" => attach_effect::resolve,
    "DestroyAll" => destroy_all_effect::resolve,
    "DamageAll" => damage_all_effect::resolve,
    "PumpAll" => pump_all_effect::resolve,
    "TapAll" => tap_all_effect::resolve,
    "TapOrUntapAll" => tap_or_untap_all_effect::resolve,
    "UntapAll" => untap_all_effect::resolve,
    "Tap" => tap_effect::resolve,
    "TapOrUntap" => tap_or_untap_effect::resolve,
    "Untap" => untap_effect::resolve,
    "LifeSet" => life_set_effect::resolve,
    "LifeExchange" => life_exchange_effect::resolve,
    "GameWin" => game_win_effect::resolve,
    "GameLoss" => game_loss_effect::resolve,
    "GameDraw" => game_draw_effect::resolve,
    "AddTurn" => add_turn_effect::resolve,
    "ActivateAbility" => activate_ability_effect::resolve,
    "Fog" => fog_effect::resolve,
    "ReverseTurnOrder" => reverse_turn_order_effect::resolve,
    "EndCombatPhase" => end_combat_phase_effect::resolve,
    "EndTurn" => end_turn_effect::resolve,
    "PowerExchange" => power_exchange_effect::resolve,
    "BecomeMonarch" => become_monarch_effect::resolve,
    "TakeInitiative" => take_initiative_effect::resolve,
    "SkipTurn" => skip_turn_effect::resolve,
    "SkipPhase" => skip_phase_effect::resolve,
    "AddPhase" => add_phase_effect::resolve,
    "Phases" => phases_effect::resolve,
    "Regenerate" => regenerate_effect::resolve,
    "Play" => play_effect::resolve,
    "Animate" => animate_effect::resolve,
    "AnimateAll" => animate_all_effect::resolve,
    "Balance" => balance_effect::resolve,
    "ChooseCard" => choose_card_effect::resolve,
    "ChooseColor" => choose_color_effect::resolve,
    "ChooseDirection" => choose_direction_effect::resolve,
    "ChooseEvenOdd" => choose_even_odd_effect::resolve,
    "Clone" => clone_effect::resolve,
    "Connive" => connive_effect::resolve,
    "ControlGainVariant" => control_gain_variant_effect::resolve,
    "RepeatEach" => repeat_each_effect::resolve,
    "Shuffle" => shuffle_effect::resolve,
    "PutCounterAll" => counters_put_all_effect::resolve,
    "CountersPutOrRemove" => counters_put_or_remove_effect::resolve,
    "EachDamage" => each_damage_effect::resolve,
    "Effect" => effect_effect::resolve,
    "DelayedTrigger" => delayed_trigger_effect::resolve,
    "DrainMana" => drain_mana_effect::resolve,
    "RemoveFromCombat" => remove_from_combat_effect::resolve,
    "Detain" => detain_effect::resolve,
    "Goad" => goad_effect::resolve,
    "ChoosePlayer" => choose_player_effect::resolve,
    "ChooseSource" => choose_source_effect::resolve,
    "ChooseType" => choose_type_effect::resolve,
    "NameCard" => name_card_effect::resolve,
    "ChooseNumber" => choose_number_effect::resolve,
    "DigUntil" => dig_until_effect::resolve,
    "FlipACoin" => flip_a_coin_effect::resolve,
    "Explore" => explore_effect::resolve,
    "RollDice" => roll_dice_effect::resolve,
    "Protection" => protection_effect::resolve,
    "ProtectionAll" => protection_all_effect::resolve,
    "PreventDamage" => prevent_damage_effect::resolve,
    "Proliferate" => proliferate_effect::resolve,
    "MoveCounter" => move_counter_effect::resolve,
    "TimeTravel" => time_travel_effect::resolve,
    "MustBlock" => must_block_effect::resolve,
    "CopySpellAbility" => copy_spell_ability_effect::resolve,
    "TwoPiles" => two_piles_effect::resolve,
    "Encode" => encode_effect::resolve,
}

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
    /// Pluggable RNG for game effects (shuffles, coin flips, dice rolls).
    /// Parity tests inject a JavaRandom-backed implementation; normal gameplay
    /// uses the default ThreadRngAdapter.
    pub rng: &'a mut dyn crate::game_rng::GameRng,
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

/// Check ConditionPresent$ / ConditionZone$ / ConditionCompare$ against game state.
/// Returns true if the condition is met (or if no condition params exist).
fn check_condition_present(
    game: &GameState,
    sa: &SpellAbility,
    player: PlayerId,
    source_id: CardId,
) -> bool {
    let condition = match sa.params.get("ConditionPresent") {
        Some(c) => c.clone(),
        None => return true, // No condition — always passes
    };

    let zone_str = sa
        .params
        .get("ConditionZone")
        .map(String::as_str)
        .unwrap_or("Battlefield");

    let zone = match zone_str.to_ascii_lowercase().as_str() {
        "graveyard" => ZoneType::Graveyard,
        "hand" => ZoneType::Hand,
        "exile" => ZoneType::Exile,
        "library" => ZoneType::Library,
        _ => ZoneType::Battlefield,
    };

    // Parse condition alternatives (comma-separated)
    let alternatives: Vec<&str> = condition.split(',').map(|s| s.trim()).collect();

    // Count matching cards
    let cards = game.cards_in_zone(zone, player);
    let count = cards
        .iter()
        .filter(|&&cid| {
            if cid == source_id {
                return false; // Don't count self
            }
            let card = game.card(cid);
            alternatives.iter().any(|alt| {
                let (base, qualifier) = if let Some((b, q)) = alt.split_once('.') {
                    (b, Some(q))
                } else {
                    (*alt, None)
                };
                let type_ok = match base.to_ascii_lowercase().as_str() {
                    "card" => true,
                    "creature" => card.is_creature(),
                    "instant" => card.type_line.is_instant(),
                    "sorcery" => card.type_line.is_sorcery(),
                    "artifact" => card.type_line.is_artifact(),
                    "enchantment" => card.type_line.is_enchantment(),
                    "land" => card.is_land(),
                    "planeswalker" => card.type_line.is_planeswalker(),
                    _ => card.type_line.has_subtype(base),
                };
                if !type_ok {
                    return false;
                }
                // Check qualifier
                if let Some(q) = qualifier {
                    match q.to_ascii_lowercase().as_str() {
                        "youctrl" | "youown" => card.controller == player,
                        "oppctrl" => card.controller != player,
                        _ => true,
                    }
                } else {
                    true
                }
            })
        })
        .count() as i32;

    // Check ConditionCompare$ (e.g. "GE2", "EQ0")
    if let Some(compare) = sa.params.get("ConditionCompare") {
        compare_condition(count, compare)
    } else {
        count > 0
    }
}

/// Compare a value against a condition string (e.g. "GE2", "EQ0", "LE3").
fn compare_condition(value: i32, compare: &str) -> bool {
    if let Some(n) = compare
        .strip_prefix("GE")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value >= n;
    }
    if let Some(n) = compare
        .strip_prefix("GT")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value > n;
    }
    if let Some(n) = compare
        .strip_prefix("LE")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value <= n;
    }
    if let Some(n) = compare
        .strip_prefix("LT")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value < n;
    }
    if let Some(n) = compare
        .strip_prefix("EQ")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value == n;
    }
    if let Some(n) = compare
        .strip_prefix("NE")
        .and_then(|s| s.parse::<i32>().ok())
    {
        return value != n;
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

    // Check ConditionPresent$ / ConditionZone$ / ConditionCompare$ conditions
    let source_id = match sa.source {
        Some(id) => id,
        None => return, // No source card — skip condition check
    };
    if !check_condition_present(ctx.game, sa, sa.activating_player, source_id) {
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
        if let Some(unless_cost) = sa
            .params
            .get("UnlessCost")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            resolve_effect_with_unless_cost(ctx, sa, unless_cost);
        } else {
            resolve_effect_once(ctx, sa);
        }
    }
}

/// Resolve a SpellAbility with Java-style `UnlessCost` payment flow.
/// Mirrors `AbilityUtils.handleUnlessCost(...)` for the core branch:
/// if a payer pays the cost, resolution is gated by `UnlessSwitched`.
fn resolve_effect_with_unless_cost(ctx: &mut EffectContext, sa: &SpellAbility, unless_cost: &str) {
    let source = match sa.source {
        Some(cid) => cid,
        None => {
            resolve_effect_once(ctx, sa);
            return;
        }
    };
    let cost = parse_cost(unless_cost);
    let payers = resolve_unless_payers(sa, ctx.game);
    // Java parity: payCostToPreventEffect → payWithDeterministicDecision →
    // CostPutCounter.visit() always pays from source without calling confirm().
    // No extra RNG/prompt consumption — just attempt to pay if able.
    let mut already_paid = false;
    for payer in payers {
        if ctx.game.player(payer).has_lost {
            continue;
        }
        if !crate::cost::can_pay_with_ability(
            &cost,
            ctx.game,
            &ctx.mana_pools[payer.index()],
            source,
            payer,
            Some(sa),
        ) {
            continue;
        }
        if try_pay_unless_cost(ctx, sa, source, payer, &cost) {
            already_paid = true;
            break;
        }
    }

    let is_switched = sa.params.contains_key("UnlessSwitched");
    if already_paid == is_switched {
        resolve_effect_once(ctx, sa);
    }
}

fn resolve_unless_payers(sa: &SpellAbility, game: &GameState) -> Vec<PlayerId> {
    let pays = sa
        .params
        .get("UnlessPayer")
        .map(|s| s.as_str())
        .unwrap_or("TargetedController");
    if pays.eq_ignore_ascii_case("TargetedController") {
        if let Some(pid) = sa.target_chosen.target_player {
            vec![pid]
        } else {
            vec![game.opponent_of(sa.activating_player)]
        }
    } else {
        resolve_defined_players(pays, sa.activating_player, game)
    }
}

fn try_pay_unless_cost(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    source: CardId,
    payer: PlayerId,
    cost: &Cost,
) -> bool {
    if !crate::cost::can_pay_with_ability(
        cost,
        ctx.game,
        &ctx.mana_pools[payer.index()],
        source,
        payer,
        Some(sa),
    ) {
        return false;
    }

    // Java parity: UnlessCost payment goes through CostPayment.payComputerCosts()
    // with DeterministicCostDecision, which calls confirm() on certain cost parts.
    // confirm() skips the prompt if isSpellPaymentContext(ability) is true.
    // Mirrors Java's DeterministicCostPlumbing.isSpellPaymentContext().
    let spell_context = is_spell_payment_context(sa, ctx.game);

    // Pre-check that all cost parts are supported before executing any,
    // to avoid partial side-effects (damage/life loss) that can't be rolled back.
    for part in &cost.parts {
        match part {
            CostPart::DamageYou(_)
            | CostPart::PayLife(_)
            | CostPart::Mana(_)
            | CostPart::PayEnergy(_)
            | CostPart::PayShards(_)
            | CostPart::Draw(_)
            | CostPart::Mill(_)
            | CostPart::AddCounter { .. } => {}
            _ => {
                return false;
            }
        }
    }

    // Java parity: DeterministicCostDecision.confirm() calls confirmPayment()
    // for certain cost parts when NOT in a spell payment context.
    // If the agent declines, the entire UnlessCost payment fails.
    if !spell_context {
        for part in &cost.parts {
            let should_ask = match part {
                // Java: CostDamage.visit() → confirm(cost, true)
                CostPart::DamageYou(_) => true,
                // Java: CostPayLife.visit() → confirm(cost, !isMandatory())
                // UnlessCost is never mandatory
                CostPart::PayLife(_) => true,
                // Java: CostDraw.visit() → confirm(cost, true)
                CostPart::Draw(_) => true,
                // Java: CostMill.visit() → confirm(cost, true)
                CostPart::Mill(_) => true,
                // Java: CostAddMana.visit() → confirm(cost, true)
                CostPart::AddMana { .. } => true,
                // Java: CostPayEnergy, CostPayShards, CostPutCounter, CostPartMana → no confirm
                _ => false,
            };
            if should_ask {
                let card_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());
                let api = sa.api.as_deref();
                let kind = unless_cost_part_kind(part);
                let message = format!(
                    "Pay {} cost for {}?",
                    kind,
                    card_name.as_deref().unwrap_or("unknown")
                );
                if !ctx.agents[payer.index()].confirm_payment(
                    payer,
                    kind,
                    &message,
                    card_name.as_deref(),
                    api,
                ) {
                    return false;
                }
            }
        }
    }

    for part in &cost.parts {
        match part {
            CostPart::DamageYou(amount) => {
                ctx.game.deal_damage_to_player(payer, *amount);
                ctx.trigger_handler.run_trigger(
                    TriggerType::DamageDone,
                    RunParams {
                        damage_target_player: Some(payer),
                        damage_amount: Some(*amount),
                        is_combat_damage: Some(false),
                        ..Default::default()
                    },
                    false,
                );
            }
            CostPart::PayLife(amount) => {
                ctx.game.player_mut(payer).lose_life(*amount);
                ctx.trigger_handler.run_trigger(
                    TriggerType::LifeLost,
                    RunParams {
                        player: Some(payer),
                        life_amount: Some(*amount),
                        ..Default::default()
                    },
                    false,
                );
            }
            CostPart::Mana(mana_cost) => {
                let _ = ctx.mana_pools[payer.index()].try_pay(mana_cost);
            }
            CostPart::PayEnergy(amount) => {
                ctx.game.player_mut(payer).energy_counters -= *amount;
            }
            CostPart::PayShards(amount) => {
                ctx.game.player_mut(payer).mana_shards -= *amount;
            }
            CostPart::Draw(amount) => {
                for _ in 0..*amount {
                    ctx.game.draw_card(payer);
                }
            }
            CostPart::Mill(amount) => {
                for _ in 0..*amount {
                    if let Some(top) = ctx.game.zone_mut(ZoneType::Library, payer).take_top() {
                        ctx.game.move_card(top, ZoneType::Graveyard, payer);
                        ctx.trigger_handler.run_trigger(
                            TriggerType::Milled,
                            RunParams {
                                card: Some(top),
                                player: Some(payer),
                                ..Default::default()
                            },
                            false,
                        );
                        emit_zone_trigger(
                            &mut ctx.trigger_handler,
                            top,
                            ZoneType::Library,
                            ZoneType::Graveyard,
                        );
                    }
                }
            }
            CostPart::AddCounter {
                amount,
                counter_type,
            } => {
                // Put counters on the source permanent (e.g. Fabricate UnlessCost).
                // Mirrors Java CostPutCounter payment.
                ctx.game.card_mut(source).add_counter(counter_type, *amount);
            }
            _ => {
                // Unsupported UnlessCost part in effect resolution path.
                return false;
            }
        }
    }
    true
}

/// Pay the merged cumulative upkeep cost. Mirrors Java's payCostToPreventEffect
/// flow for cumulative upkeep in SacrificeEffect. Supports FlipCoin, Mill, Mana,
/// and other standard cost parts.
pub(super) fn try_pay_cumulative_upkeep(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    source: CardId,
    payer: PlayerId,
    cost: &Cost,
) -> bool {
    // Check payability
    if !crate::cost::can_pay_with_ability(
        cost,
        ctx.game,
        &ctx.mana_pools[payer.index()],
        source,
        payer,
        Some(sa),
    ) {
        return false;
    }

    // Cumulative upkeep is never a spell context — always confirm
    for part in &cost.parts {
        let should_ask = match part {
            CostPart::DamageYou(_) => true,
            CostPart::PayLife(_) => true,
            CostPart::Draw(_) => true,
            CostPart::Mill(_) => true,
            CostPart::AddMana { .. } => true,
            // Java: CostFlipCoin.visit() → confirm(cost, true)
            CostPart::FlipCoin(_) => true,
            _ => false,
        };
        if should_ask {
            let card_name = ctx.game.card(source).card_name.clone();
            let api = sa.api.as_deref();
            let kind = unless_cost_part_kind(part);
            let message = format!("Pay {} cost for {}?", kind, card_name);
            let result = ctx.agents[payer.index()].confirm_payment(
                payer,
                kind,
                &message,
                Some(&card_name),
                api,
            );
            if !result {
                return false;
            }
        }
    }

    // Pay each cost part
    for part in &cost.parts {
        match part {
            CostPart::FlipCoin(amount) => {
                let resolved_amount =
                    crate::cost::resolve_dynamic_amount(ctx.game, source, payer, *amount);
                for _ in 0..resolved_amount {
                    let source_name = ctx.game.card(source).card_name.clone();
                    let called_heads = ctx.agents[payer.index()].choose_binary(
                        payer,
                        "Call the coin flip",
                        crate::agent::BinaryChoiceKind::HeadsOrTails,
                        None,
                        Some(&source_name),
                        None,
                    );
                    let is_heads = ctx.rng.next_int(2) == 0;
                    let won = called_heads == is_heads;
                    ctx.trigger_handler.run_trigger(
                        TriggerType::FlippedCoin,
                        RunParams {
                            player: Some(payer),
                            coin_flip_won: Some(won),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
            CostPart::DamageYou(amount) => {
                ctx.game.deal_damage_to_player(payer, *amount);
                ctx.trigger_handler.run_trigger(
                    TriggerType::DamageDone,
                    RunParams {
                        damage_target_player: Some(payer),
                        damage_amount: Some(*amount),
                        is_combat_damage: Some(false),
                        ..Default::default()
                    },
                    false,
                );
            }
            CostPart::PayLife(amount) => {
                ctx.game.player_mut(payer).lose_life(*amount);
                ctx.trigger_handler.run_trigger(
                    TriggerType::LifeLost,
                    RunParams {
                        player: Some(payer),
                        life_amount: Some(*amount),
                        ..Default::default()
                    },
                    false,
                );
            }
            CostPart::Mana(mana_cost) => {
                let _ = ctx.mana_pools[payer.index()].try_pay(mana_cost);
            }
            CostPart::Mill(amount) => {
                for _ in 0..*amount {
                    if let Some(top) = ctx.game.zone_mut(ZoneType::Library, payer).take_top() {
                        ctx.game.move_card(top, ZoneType::Graveyard, payer);
                        ctx.trigger_handler.run_trigger(
                            TriggerType::Milled,
                            RunParams {
                                card: Some(top),
                                player: Some(payer),
                                ..Default::default()
                            },
                            false,
                        );
                        emit_zone_trigger(
                            &mut ctx.trigger_handler,
                            top,
                            ZoneType::Library,
                            ZoneType::Graveyard,
                        );
                    }
                }
            }
            CostPart::AddCounter {
                amount,
                counter_type,
            } => {
                ctx.game.card_mut(source).add_counter(counter_type, *amount);
            }
            _ => {
                // Unsupported cost part
                return false;
            }
        }
    }
    true
}

/// Mirrors Java's `DeterministicCostPlumbing.isSpellPaymentContext()`.
/// Returns true if the SA represents a spell being cast (not a trigger/activated ability).
fn is_spell_payment_context(sa: &SpellAbility, game: &GameState) -> bool {
    if sa.is_spell {
        return true;
    }
    if let Some(cid) = sa.source {
        let card = game.card(cid);
        if card.type_line.is_instant() || card.type_line.is_sorcery() {
            return true;
        }
    }
    false
}

/// Cost part kind label for UnlessCost confirm_payment prompts.
fn unless_cost_part_kind(part: &CostPart) -> &'static str {
    match part {
        CostPart::DamageYou(_) => "DamageYou",
        CostPart::PayLife(_) => "PayLife",
        CostPart::Draw(_) => "Draw",
        CostPart::Mill(_) => "Mill",
        CostPart::AddMana { .. } => "AddMana",
        _ => "Cost",
    }
}

/// Format a human-readable message for an UnlessCost prompt.
/// e.g. "AddCounter<2/P1P1>" → "Put 2 +1/+1 counters instead?"
fn cost_part_kind(part: &CostPart) -> &'static str {
    match part {
        CostPart::Mana(_) => "Mana",
        CostPart::PayLife(_) => "PayLife",
        CostPart::DamageYou(_) => "DamageYou",
        CostPart::PayEnergy(_) => "PayEnergy",
        CostPart::PayShards(_) => "PayShards",
        CostPart::Draw(_) => "Draw",
        CostPart::Mill(_) => "Mill",
        _ => "Cost",
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
    } else if ability.contains("DrainMana") {
        "DrainMana"
    } else if ability.contains("ManaReflected") {
        "ManaReflected"
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
    } else if ability.contains("GainControl") || ability.contains("ControlGain") {
        "ControlGain"
    } else if ability.contains("$ Fight") {
        "Fight"
    } else if ability.contains("$ Connive") {
        "Connive"
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
    } else if ability.contains("TapOrUntapAll") {
        "TapOrUntapAll"
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
    } else if ability.contains("ActivateAbility") {
        "ActivateAbility"
    } else if ability.contains("$ Fog") {
        "Fog"
    } else if ability.contains("TapOrUntap") {
        "TapOrUntap"
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
    } else if ability.contains("ChooseDirection") {
        "ChooseDirection"
    } else if ability.contains("ChooseEvenOdd") {
        "ChooseEvenOdd"
    } else if ability.contains("$ Clone") {
        "Clone"
    } else if ability.contains("ControlGainVariant") {
        "ControlGainVariant"
    } else if ability.contains("RepeatEach") {
        "RepeatEach"
    // High-priority effects (issue #53) — PutCounterAll before PutCounter, EachDamage before DealDamage
    } else if ability.contains("PutCounterAll") {
        "PutCounterAll"
    } else if ability.contains("CountersPutOrRemove") {
        "CountersPutOrRemove"
    } else if ability.contains("EachDamage") {
        "EachDamage"
    } else if ability.contains("$ Effect") {
        "Effect"
    } else if ability.contains("DelayedTrigger") {
        "DelayedTrigger"
    } else if ability.contains("$ Shuffle") {
        "Shuffle"
    } else if ability.contains("RemoveFromCombat") {
        "RemoveFromCombat"
    } else if ability.contains("$ Detain") {
        "Detain"
    } else if ability.contains("$ Goad") {
        "Goad"
    } else if ability.contains("ChoosePlayer") {
        "ChoosePlayer"
    } else if ability.contains("ChooseSource") {
        "ChooseSource"
    } else if ability.contains("ChooseType") {
        "ChooseType"
    } else if ability.contains("NameCard") {
        "NameCard"
    } else if ability.contains("ChooseNumber") {
        "ChooseNumber"
    } else if ability.contains("DigUntil") {
        "DigUntil"
    } else if ability.contains("FlipACoin") {
        "FlipACoin"
    } else if ability.contains("$ Explore") {
        "Explore"
    } else if ability.contains("RollDice") {
        "RollDice"
    // High-priority effects (issue #53, Batch 4) — ProtectionAll before Protection
    } else if ability.contains("ProtectionAll") {
        "ProtectionAll"
    } else if ability.contains("$ Protection") {
        "Protection"
    } else if ability.contains("PreventDamage") {
        "PreventDamage"
    } else if ability.contains("$ Proliferate") {
        "Proliferate"
    } else if ability.contains("MoveCounter") {
        "MoveCounter"
    } else if ability.contains("TimeTravel") {
        "TimeTravel"
    } else if ability.contains("MustBlock") {
        "MustBlock"
    // High-priority effects (issue #53, Batch 5)
    } else if ability.contains("CopySpellAbility") {
        "CopySpellAbility"
    } else if ability.contains("TwoPiles") {
        "TwoPiles"
    } else if ability.contains("$ Encode") {
        "Encode"
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

    // Check if it's the X mana cost value directly
    if val_str.trim() == "X" {
        // First check if there's an SVar named "X" on the source card
        if let Some(source_id) = sa.source {
            if let Some(svar_expr) = game.card(source_id).svars.get("X") {
                if svar_expr.starts_with("Count$") {
                    return resolve_count_svar_for_sa(
                        svar_expr,
                        game,
                        source_id,
                        sa.activating_player,
                        sa,
                    );
                }
                // TriggeredCard$CardPower / TriggeredCard$CardToughness — LKI
                // Use lki_power (captured at zone-change time) as primary source,
                // falling back to the periodic snapshot if lki_power is 0 and
                // the snapshot has a non-zero value. This handles cards that
                // entered the battlefield after the last checkpoint.
                if svar_expr == "TriggeredCard$CardPower" {
                    if let Some(trigger_src) = sa.trigger_source {
                        let lki = game.card(trigger_src).lki_power;
                        if lki != 0 {
                            return lki;
                        }
                        // lki_power is 0 — could be base 0/X creature or power
                        // was reduced to 0 by effects. Check snapshot for
                        // pre-resolution state.
                        if let Some(snapshot) = game.get_lki_snapshot(trigger_src) {
                            return snapshot.power;
                        }
                        return lki;
                    }
                    return 0;
                }
                if svar_expr == "TriggeredCard$CardToughness" {
                    if let Some(trigger_src) = sa.trigger_source {
                        let lki = game.card(trigger_src).lki_toughness;
                        if lki != 0 {
                            return lki;
                        }
                        if let Some(snapshot) = game.get_lki_snapshot(trigger_src) {
                            return snapshot.toughness;
                        }
                        return game.card(trigger_src).lki_toughness;
                    }
                    return 0;
                }
                return evaluate_svar(svar_expr, sa);
            }
        }
        // Otherwise use x_mana_cost_paid directly
        return sa.x_mana_cost_paid as i32;
    }

    // It's an SVar reference — look it up on the source card
    if let Some(source_id) = sa.source {
        if let Some(svar_expr) = game.card(source_id).svars.get(val_str.trim()) {
            // Game-aware SVar resolution for patterns that need GameState.
            if svar_expr.starts_with("Count$") {
                return resolve_count_svar_for_sa(
                    svar_expr,
                    game,
                    source_id,
                    sa.activating_player,
                    sa,
                );
            }
            // TriggeredCard$CardPower / TriggeredCard$CardToughness — LKI of the triggered card.
            // Mirrors Java AbilityUtils: TriggeredCard → Card, then Card$CardPower.
            if svar_expr == "TriggeredCard$CardPower" {
                if let Some(trigger_src) = sa.trigger_source {
                    let lki = game.card(trigger_src).lki_power;
                    if lki != 0 {
                        return lki;
                    }
                    if let Some(snapshot) = game.get_lki_snapshot(trigger_src) {
                        return snapshot.power;
                    }
                    return lki;
                }
                return 0;
            }
            if svar_expr == "TriggeredCard$CardToughness" {
                if let Some(trigger_src) = sa.trigger_source {
                    if let Some(snapshot) = game.get_lki_snapshot(trigger_src) {
                        return snapshot.toughness;
                    }
                    return game.card(trigger_src).lki_toughness;
                }
                return 0;
            }
            return evaluate_svar(svar_expr, sa);
        }
    }

    default
}

/// Evaluate a simple SVar expression.
/// Supports `Count$Kicked.A.B` (returns A if kicked, B otherwise)
/// and `Count$KickedCount` (returns the multikicker count).
pub fn evaluate_svar(expr: &str, sa: &SpellAbility) -> i32 {
    // X mana cost — return the value of X paid when casting
    if expr == "Count$xPaid" || expr == "Count$XPaid" {
        return sa.x_mana_cost_paid as i32;
    }
    // Converge/Sunburst — handled in resolve_numeric_svar (needs GameState)
    if expr == "Count$Converge" || expr == "Count$Sunburst" {
        return 0; // Fallback; game-aware path in resolve_numeric_svar handles this
    }
    if expr == "Count$TriggerRememberAmount" {
        return sa.trigger_remembered_amount;
    }
    // TriggerCount$Amount — number of objects that matched the trigger event.
    // For per-event triggers (ChangesZoneAll batched as individual fires), this is 1.
    if expr == "TriggerCount$Amount" {
        return sa.trigger_remembered_amount.max(1);
    }
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
    // Number$N — literal numeric SVar (e.g. "Number$2" set by LoseLife for AFLifeLost)
    if let Some(rest) = expr.strip_prefix("Number$") {
        return rest.trim().parse::<i32>().unwrap_or(0);
    }
    // Fallback: try parsing as integer
    expr.parse::<i32>().unwrap_or(0)
}

/// Resolve a Count$ SVar expression that requires game state access.
/// Handles patterns like `Count$Valid Forest.YouCtrl`, `Count$Converge`,
/// `Count$CardPower`, etc.
/// Mirrors Java's `AbilityUtils.calculateAmount()` for Count$ expressions.
pub fn resolve_count_svar(
    expr: &str,
    game: &GameState,
    source_id: CardId,
    controller: PlayerId,
) -> i32 {
    resolve_count_svar_for_sa(
        expr,
        game,
        source_id,
        controller,
        &crate::spellability::SpellAbility::new_simple(Some(source_id), controller, ""),
    )
}

pub fn resolve_count_svar_for_sa(
    expr: &str,
    game: &GameState,
    source_id: CardId,
    controller: PlayerId,
    sa: &SpellAbility,
) -> i32 {
    use forge_foundation::ZoneType;

    if expr == "Count$TriggerRememberAmount" {
        return sa.trigger_remembered_amount;
    }

    if expr == "Count$Converge" || expr == "Count$Sunburst" {
        return game.card(source_id).sunburst_count();
    }

    if let Some(rest) = expr.strip_prefix("Count$OptionalGenericCostPaid.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let paid_val = parts[0].parse::<i32>().unwrap_or(1);
            let unpaid_val = parts[1].parse::<i32>().unwrap_or(0);
            return if sa.optional_generic_cost_paid {
                paid_val
            } else {
                unpaid_val
            };
        }
    }

    // Count$PromisedGift.A.B — return A when gift promised, else B.
    if let Some(rest) = expr.strip_prefix("Count$PromisedGift.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if parts.len() == 2 {
            let promised_val = parts[0].parse::<i32>().unwrap_or(1);
            let not_promised_val = parts[1].parse::<i32>().unwrap_or(0);
            return if game.card(source_id).promised_gift.is_some() {
                promised_val
            } else {
                not_promised_val
            };
        }
    }
    if expr == "Count$PromisedGift" {
        return if game.card(source_id).promised_gift.is_some() {
            1
        } else {
            0
        };
    }

    // Count$Valid TYPE.QUALIFIERS — count permanents matching filter
    // Count$Valid TYPE.QUALIFIERS$GreatestCardPower — greatest power among matching creatures
    if let Some(filter_str) = expr.strip_prefix("Count$Valid ") {
        // Check for $GreatestCardPower suffix
        let (filter_str, greatest_power) =
            if let Some(base) = filter_str.strip_suffix("$GreatestCardPower") {
                (base, true)
            } else {
                (filter_str, false)
            };

        let battlefield = game.cards_in_zone(ZoneType::Battlefield, controller);
        // Also check opponent's battlefield for non-YouCtrl filters
        let opp = game.opponent_of(controller);
        let opp_battlefield = game.cards_in_zone(ZoneType::Battlefield, opp);

        let has_you_ctrl = filter_str.contains("YouCtrl") || filter_str.contains("YouControl");

        let cards_to_check: Vec<CardId> = if has_you_ctrl {
            battlefield.to_vec()
        } else {
            battlefield
                .iter()
                .chain(opp_battlefield.iter())
                .copied()
                .collect()
        };

        let chosen_type = game.card(source_id).chosen_type.clone();
        if greatest_power {
            // Return the greatest power among matching creatures
            let mut max_power = 0;
            for &cid in &cards_to_check {
                let card = game.card(cid);
                if valid_card_matches_with_source(
                    filter_str,
                    card,
                    controller,
                    source_id,
                    chosen_type.as_deref(),
                ) {
                    max_power = max_power.max(card.power());
                }
            }
            return max_power;
        } else {
            let mut count = 0;
            for &cid in &cards_to_check {
                let card = game.card(cid);
                if valid_card_matches_with_source(
                    filter_str,
                    card,
                    controller,
                    source_id,
                    chosen_type.as_deref(),
                ) {
                    count += 1;
                }
            }
            return count;
        }
    }

    // Count$Devotion.COLOR — count mana symbols of a color among permanents you control.
    // Mirrors Java's `CardFactoryUtil.xCount()` Devotion case.
    if let Some(color_str) = expr.strip_prefix("Count$Devotion.") {
        let color_mask: u16 = match color_str.to_uppercase().as_str() {
            "W" | "WHITE" => forge_foundation::ManaAtom::WHITE,
            "U" | "BLUE" => forge_foundation::ManaAtom::BLUE,
            "B" | "BLACK" => forge_foundation::ManaAtom::BLACK,
            "R" | "RED" => forge_foundation::ManaAtom::RED,
            "G" | "GREEN" => forge_foundation::ManaAtom::GREEN,
            _ => 0,
        };
        if color_mask != 0 {
            let battlefield = game.cards_in_zone(ZoneType::Battlefield, controller);
            let mut count = 0i32;
            for &cid in battlefield {
                let card = game.card(cid);
                for shard in card.mana_cost.shards() {
                    if (shard.shard() & color_mask) != 0 {
                        count += 1;
                    }
                }
            }
            return count;
        }
    }

    // Count$Compare SVAR OPTHRESHOLD.IFTRUE.IFFALSE
    // e.g. Count$Compare Y GE1.3.1  → if Y >= 1 then 3 else 1
    if let Some(rest) = expr.strip_prefix("Count$Compare ") {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let svar_name = parts[0];
            let cond_parts: Vec<&str> = parts[1].splitn(3, '.').collect();
            if cond_parts.len() == 3 {
                // Resolve the referenced SVar
                let svar_val = if let Some(svar_expr) = game.card(source_id).svars.get(svar_name) {
                    if svar_expr.starts_with("Count$") {
                        resolve_count_svar_for_sa(svar_expr, game, source_id, controller, sa)
                    } else {
                        svar_expr.parse::<i32>().unwrap_or(0)
                    }
                } else {
                    svar_name.parse::<i32>().unwrap_or(0)
                };

                // Parse operator + threshold from cond_parts[0], e.g. "GE1"
                let cond = cond_parts[0];
                let (op, threshold) = if let Some(t) = cond.strip_prefix("GE") {
                    ("GE", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("GT") {
                    ("GT", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("LE") {
                    ("LE", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("LT") {
                    ("LT", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("EQ") {
                    ("EQ", t.parse::<i32>().unwrap_or(0))
                } else if let Some(t) = cond.strip_prefix("NE") {
                    ("NE", t.parse::<i32>().unwrap_or(0))
                } else {
                    ("GE", 0)
                };

                let result = match op {
                    "GE" => svar_val >= threshold,
                    "GT" => svar_val > threshold,
                    "LE" => svar_val <= threshold,
                    "LT" => svar_val < threshold,
                    "EQ" => svar_val == threshold,
                    "NE" => svar_val != threshold,
                    _ => false,
                };

                let if_true = cond_parts[1].parse::<i32>().unwrap_or(0);
                let if_false = cond_parts[2].parse::<i32>().unwrap_or(0);
                return if result { if_true } else { if_false };
            }
        }
    }

    // Count$CardPower — power of the source card
    if expr == "Count$CardPower" {
        return game.card(source_id).power();
    }
    // Count$CardToughness
    if expr == "Count$CardToughness" {
        return game.card(source_id).toughness();
    }
    // Count$CardCounters.TYPE
    if let Some(counter_type) = expr.strip_prefix("Count$CardCounters.") {
        let ct = crate::ability::effects::parse_counter_type(counter_type);
        return *game.card(source_id).counters.get(&ct).unwrap_or(&0);
    }

    // Count$TotalDamageDoneByThisTurn — total damage dealt by the source card this turn.
    // Mirrors Java's Card.getTotalDamageDoneBy() via DamageHistory.
    if expr == "Count$TotalDamageDoneByThisTurn" {
        return game.card(source_id).total_damage_done_this_turn;
    }

    // Fallback
    expr.parse::<i32>().unwrap_or(1)
}

/// Check if a card matches a validity filter string like "Forest.YouCtrl".
fn valid_card_matches_with_source(
    filter: &str,
    card: &crate::card::CardInstance,
    controller: PlayerId,
    source_id: CardId,
    chosen_type: Option<&str>,
) -> bool {
    let parts: Vec<&str> = filter.split('.').collect();
    let base_type = parts.first().copied().unwrap_or("");

    // Check base type
    let type_ok = match base_type {
        "Creature" => card.is_creature(),
        "Land" => card.is_land(),
        "Artifact" => card.type_line.is_artifact(),
        "Enchantment" => card.type_line.is_enchantment(),
        "Planeswalker" => card.type_line.is_planeswalker(),
        "Permanent" | "Card" => true,
        // Subtypes (Forest, Island, Goblin, etc.)
        _ => card.type_line.has_subtype(base_type),
    };
    if !type_ok {
        return false;
    }

    // Check qualifiers (split by '.' and '+')
    for &dot_qual in &parts[1..] {
        for sub_qual in dot_qual.split('+') {
            let sub_qual = sub_qual.trim();
            if sub_qual.eq_ignore_ascii_case("YouCtrl")
                || sub_qual.eq_ignore_ascii_case("YouControl")
            {
                if card.controller != controller {
                    return false;
                }
            } else if sub_qual.eq_ignore_ascii_case("Self") {
                if card.id != source_id {
                    return false;
                }
            } else if sub_qual.eq_ignore_ascii_case("Other") {
                // handled by caller if needed
            } else if sub_qual.eq_ignore_ascii_case("ChosenType") {
                // Card must have the source card's chosen creature type as a subtype.
                // Mirrors Java CardTraitBase.isValid() ChosenType qualifier.
                match chosen_type {
                    Some(ct) if card.type_line.has_subtype(ct) => {}
                    _ => return false,
                }
            } else if sub_qual.starts_with("counters_") {
                // Parse "counters_GE1_P1P1", "counters_EQ0_P1P1", etc.
                if !check_counter_qualifier(card, sub_qual) {
                    return false;
                }
            }
        }
    }
    true
}

/// Check a counter qualifier like "counters_GE1_P1P1".
fn check_counter_qualifier(card: &crate::card::CardInstance, qual: &str) -> bool {
    let rest = match qual.strip_prefix("counters_") {
        Some(r) => r,
        None => return true,
    };
    // Split into OP+THRESHOLD and COUNTER_TYPE, e.g. "GE1_P1P1"
    let parts: Vec<&str> = rest.splitn(2, '_').collect();
    if parts.len() != 2 {
        return true;
    }
    let cond = parts[0];
    let counter_type = crate::ability::effects::parse_counter_type(parts[1]);
    let count = *card.counters.get(&counter_type).unwrap_or(&0);

    let (op, threshold) = if let Some(t) = cond.strip_prefix("GE") {
        ("GE", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("GT") {
        ("GT", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("LE") {
        ("LE", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("LT") {
        ("LT", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("EQ") {
        ("EQ", t.parse::<i32>().unwrap_or(0))
    } else if let Some(t) = cond.strip_prefix("NE") {
        ("NE", t.parse::<i32>().unwrap_or(0))
    } else {
        return true;
    };

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
pub fn matches_valid_cards(card: &CardInstance, filter: &str, activating_player: PlayerId) -> bool {
    if filter.is_empty() || filter == "Card" {
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
            card.keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case("Flying"))
                || card
                    .granted_keywords
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case("Flying"))
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
        "Artifact" => card.type_line.is_artifact(),
        "Enchantment" => card.type_line.is_enchantment(),
        "Instant" => card.type_line.is_instant(),
        "Sorcery" => card.type_line.is_sorcery(),
        "Planeswalker" => card.type_line.is_planeswalker(),
        "Permanent" => card.is_permanent(),
        "Card" => true,
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
            "Basic" => {
                if !card.type_line.is_basic() {
                    return false;
                }
            }
            "nonLand" => {
                if card.is_land() {
                    return false;
                }
            }
            "attacking" => {
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

/// Like `emit_zone_trigger` but carries LKI +1/+1 counter count.
/// Used when a creature with Modular dies so the death trigger can move
/// the correct number of counters (not just the static Modular:N value).
pub fn emit_zone_trigger_with_lki_counters(
    trigger_handler: &mut TriggerHandler,
    card_id: CardId,
    origin: ZoneType,
    destination: ZoneType,
    lki_p1p1_counters: i32,
) {
    trigger_handler.run_trigger(
        TriggerType::ChangesZone,
        RunParams {
            card: Some(card_id),
            origin: Some(origin),
            destination: Some(destination),
            lki_p1p1_counters: Some(lki_p1p1_counters),
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
