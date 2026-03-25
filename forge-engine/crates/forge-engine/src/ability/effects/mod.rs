//! Effect resolution system.
//!
//! Each effect type lives in its own file, mirroring the Java Forge
//! `ability/effects/` package (204 files). Effects are dispatched by
//! API type string extracted from the ability text.

pub mod abandon_effect;
pub mod activate_ability_effect;
pub mod add_phase_effect;
pub mod add_turn_effect;
pub mod advance_crank_effect;
pub mod airbend_effect;
pub mod alter_attribute_effect;
pub mod amass_effect;
pub mod animate_all_effect;
pub mod animate_effect;
pub mod ascend_effect;
pub mod assemble_contraption_effect;
pub mod assign_group_effect;
pub mod attach_effect;
pub mod balance_effect;
pub mod become_monarch_effect;
pub mod becomes_blocked_effect;
pub mod bid_life_effect;
pub mod blank_line_effect;
pub mod blight_effect;
pub mod block_effect;
pub mod bond_effect;
pub mod branch_effect;
pub mod camouflage_effect;
pub mod cast_from_effect;
pub mod change_combatants_effect;
pub mod change_speed_effect;
pub mod change_targets_effect;
pub mod change_text_effect;
pub mod change_x_effect;
pub mod change_zone_all_effect;
pub mod change_zone_effect;
pub mod change_zone_resolve_effect;
pub mod chaos_ensues_effect;
pub mod charm_effect;
pub mod choose_card_effect;
pub mod choose_card_name_effect;
pub mod choose_color_effect;
pub mod choose_direction_effect;
pub mod choose_even_odd_effect;
pub mod choose_generic_effect;
pub mod choose_number_effect;
pub mod choose_player_effect;
pub mod choose_sector_effect;
pub mod choose_source_effect;
pub mod choose_type_effect;
pub mod claim_the_prize_effect;
pub mod clash_effect;
pub mod class_level_up_effect;
pub mod clean_up_effect;
pub mod cleanup_effect;
pub mod cloak_effect;
pub mod clone_effect;
pub mod connive_effect;
pub mod control_exchange_effect;
pub mod control_exchange_variant_effect;
pub mod control_gain_effect;
pub mod control_gain_variant_effect;
pub mod control_player_effect;
pub mod control_spell_effect;
pub mod copy_permanent_effect;
pub mod copy_spell_ability_effect;
pub mod counter_effect;
pub mod counters_move_effect;
pub mod counters_multiply_effect;
pub mod counters_note_effect;
pub mod counters_proliferate_effect;
pub mod counters_put_all_effect;
pub mod counters_put_effect;
pub mod counters_put_or_remove_effect;
pub mod counters_remove_all_effect;
pub mod counters_remove_effect;
pub mod damage_all_effect;
pub mod damage_base_effect;
pub mod damage_deal_effect;
pub mod damage_each_effect;
pub mod damage_prevent_effect;
pub mod damage_resolve_effect;
pub mod day_time_effect;
pub mod debuff_effect;
pub mod delayed_trigger_effect;
pub mod destroy_all_effect;
pub mod destroy_effect;
pub mod detached_card_effect;
pub mod detain_effect;
pub mod dig_effect;
pub mod dig_multiple_effect;
pub mod dig_until_effect;
pub mod discard_effect;
pub mod discover_effect;
pub mod draft_effect;
pub mod drain_mana_effect;
pub mod draw_effect;
pub mod earthbend_effect;
pub mod effect_effect;
pub mod encode_effect;
pub mod end_combat_phase_effect;
pub mod end_turn_effect;
pub mod endure_effect;
pub mod explore_effect;
pub mod fight_effect;
pub mod flip_coin_effect;
pub mod flip_onto_battlefield_effect;
pub mod fog_effect;
pub mod game_draw_effect;
pub mod game_loss_effect;
pub mod game_win_effect;
pub mod goad_effect;
pub mod haunt_effect;
pub mod heist_effect;
pub mod immediate_trigger_effect;
pub mod incubate_effect;
pub mod intensify_effect;
pub mod internal_radiation_effect;
pub mod investigate_effect;
pub mod learn_effect;
pub mod life_exchange_effect;
pub mod life_exchange_variant_effect;
pub mod life_gain_effect;
pub mod life_lose_effect;
pub mod life_set_effect;
pub mod look_at_effect;
pub mod lose_perpetual_effect;
pub mod make_card_effect;
pub mod mana_effect;
pub mod mana_reflected_effect;
pub mod manifest_base_effect;
pub mod manifest_dread_effect;
pub mod manifest_effect;
pub mod meld_effect;
pub mod mill_effect;
pub mod move_counter_effect;
pub mod multiple_piles_effect;
pub mod must_block_effect;
pub mod mutate_effect;
pub mod name_card_effect;
pub mod open_attraction_effect;
pub mod ownership_gain_effect;
pub mod peek_and_reveal_effect;
pub mod permanent_creature_effect;
pub mod permanent_effect;
pub mod permanent_noncreature_effect;
pub mod phases_effect;
pub mod planeswalk_effect;
pub mod play_effect;
pub mod play_land_variant_effect;
pub mod plot_effect;
pub mod poison_effect;
pub mod power_exchange_effect;
pub mod prevent_damage_effect;
pub mod protect_all_effect;
pub mod protect_effect;
pub mod pump_all_effect;
pub mod pump_effect;
pub mod radiation_effect;
pub mod rearrange_top_of_library_effect;
pub mod regenerate_effect;
pub mod regeneration_effect;
pub mod remove_from_combat_effect;
pub mod remove_from_game_effect;
pub mod remove_from_match_effect;
pub mod reorder_zone_effect;
pub mod repeat_each_effect;
pub mod repeat_effect;
pub mod replace_counter_effect;
pub mod replace_damage_effect;
pub mod replace_effect;
pub mod replace_mana_effect;
pub mod replace_split_damage_effect;
pub mod replace_token_effect;
pub mod restart_game_effect;
pub mod reveal_effect;
pub mod reveal_hand_effect;
pub mod reverse_turn_order_effect;
pub mod ring_tempts_you_effect;
pub mod roll_dice_effect;
pub mod roll_planar_dice_effect;
pub mod run_chaos_effect;
pub mod sacrifice_all_effect;
pub mod sacrifice_effect;
pub mod scry_effect;
pub mod seek_effect;
pub mod set_in_motion_effect;
pub mod set_state_effect;
pub mod shuffle_effect;
pub mod skip_phase_effect;
pub mod skip_turn_effect;
pub mod store_s_var_effect;
pub mod subgame_effect;
pub mod surveil_effect;
pub mod switch_block_effect;
pub mod take_initiative_effect;
pub mod tap_all_effect;
pub mod tap_effect;
pub mod tap_or_untap_all_effect;
pub mod tap_or_untap_effect;
pub mod text_box_exchange_effect;
pub mod time_travel_effect;
pub mod token_effect;
pub mod trait_animate_effect;
pub mod trait_token_effect;
pub mod two_piles_effect;
pub mod unattach_effect;
pub mod unlock_door_effect;
pub mod untap_all_effect;
pub mod untap_effect;
pub mod venture_effect;
pub mod villainous_choice_effect;
pub mod vote_effect;
pub mod zone_exchange_effect;

// Helper modules for utility functions and zone triggers
pub mod helpers;
pub mod zone_triggers;

use std::collections::HashMap;

use forge_foundation::{CoreType, ZoneType};

use crate::ability::api_type::ApiType;
use crate::agent::PlayerAgent;
use crate::card::Card;
use crate::combat::DefenderId;
use crate::cost::{parse_cost, Cost, CostPart};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::parsing::compare::compare_expr;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::handler::TriggerHandler;

// Re-export SVar resolution functions for backward compatibility.
// These now live in the dedicated svar/ module.
pub use crate::svar::{
    evaluate_svar, resolve_count_svar, resolve_count_svar_for_sa, resolve_numeric_svar,
};

// Re-export helpers and zone_triggers for backward compatibility.
// All existing callers can continue using paths like `effects::parse_counter_type()`.
pub use helpers::*;
pub use zone_triggers::*;

// Re-export mana_atom_from_produced for convenience in effect files.
pub use crate::mana::mana_atom_from_produced;

/// Generates both `IMPLEMENTED_API_TYPES` and `resolve_effect_once` from a
/// single source of truth. Adding a new effect requires only one entry.
macro_rules! effect_dispatch {
    ( $( $api:path => $handler:path ),* $(,)? ) => {
        /// All API types that have implemented effect handlers.
        /// Used by the fuzz card pool filter to exclude cards with unimplemented effects.
        pub const IMPLEMENTED_API_TYPES: &[ApiType] = &[ $( $api ),* ];

        /// Inner dispatch for a single execution of an effect.
        fn resolve_effect_once(ctx: &mut EffectContext, sa: &SpellAbility) {
            let api_type = match sa.api {
                Some(api) => api,
                None => {
                    // Some DB/SVar helper nodes intentionally have no API payload.
                    // Java treats those as no-op leafs; avoid warning spam in parity runs.
                    return;
                }
            };
            match api_type {
                $( $api => $handler(ctx, sa), )*
                _ => {
                    let err = crate::ability::IllegalAbilityException::new(
                        format!("Unimplemented effect API type: {:?}", api_type),
                    );
                    eprintln!("{}", err);
                }
            }
        }
    };
}

effect_dispatch! {
    ApiType::DealDamage => damage_deal_effect::resolve,
    ApiType::Branch => branch_effect::resolve,
    ApiType::GainLife => life_gain_effect::resolve,
    ApiType::LoseLife => life_lose_effect::resolve,
    ApiType::PutCounter => counters_put_effect::resolve,
    ApiType::RemoveCounter => counters_remove_effect::resolve,
    ApiType::Poison => poison_effect::resolve,
    ApiType::Pump => pump_effect::resolve,
    ApiType::Destroy => destroy_effect::resolve,
    ApiType::Draw => draw_effect::resolve,
    ApiType::ChangeZoneAll => change_zone_all_effect::resolve,
    ApiType::ChangeZone => change_zone_effect::resolve,
    ApiType::SacrificeAll => sacrifice_all_effect::resolve,
    ApiType::Sacrifice => sacrifice_effect::resolve,
    ApiType::CopyPermanent => copy_permanent_effect::resolve,
    ApiType::Token => token_effect::resolve,
    ApiType::Amass => amass_effect::resolve,
    ApiType::Manifest => manifest_effect::resolve,
    ApiType::ManifestDread => manifest_dread_effect::resolve,
    ApiType::Cloak => cloak_effect::resolve,
    ApiType::Investigate => investigate_effect::resolve,
    ApiType::Incubate => incubate_effect::resolve,
    ApiType::Seek => seek_effect::resolve,
    ApiType::Learn => learn_effect::resolve,
    ApiType::Discover => discover_effect::resolve,
    ApiType::Meld => meld_effect::resolve,
    ApiType::ExchangeControl => control_exchange_effect::resolve,
    ApiType::ControlPlayer => control_player_effect::resolve,
    ApiType::Clash => clash_effect::resolve,
    ApiType::Vote => vote_effect::resolve,
    ApiType::VillainousChoice => villainous_choice_effect::resolve,
    ApiType::Ascend => ascend_effect::resolve,
    ApiType::DayTime => day_time_effect::resolve,
    ApiType::Haunt => haunt_effect::resolve,
    ApiType::Unattach => unattach_effect::resolve,
    ApiType::FlipOntoBattlefield => flip_onto_battlefield_effect::resolve,
    ApiType::ClassLevelUp => class_level_up_effect::resolve,
    ApiType::Venture => venture_effect::resolve,
    ApiType::RingTemptsYou => ring_tempts_you_effect::resolve,
    ApiType::Heist => heist_effect::resolve,
    ApiType::ImmediateTrigger => immediate_trigger_effect::resolve,
    ApiType::StoreSVar => store_s_var_effect::resolve,
    ApiType::ChangeTargets => change_targets_effect::resolve,
    ApiType::ChangeText => change_text_effect::resolve,
    ApiType::ChangeX => change_x_effect::resolve,
    ApiType::CountersMove => counters_move_effect::resolve,
    ApiType::MultiplyCounter => counters_multiply_effect::resolve,
    ApiType::CountersNote => counters_note_effect::resolve,
    ApiType::RemoveCounterAll => counters_remove_all_effect::resolve,
    ApiType::ReorderZone => reorder_zone_effect::resolve,
    ApiType::Repeat => repeat_effect::resolve,
    ApiType::ReplaceEffect => replace_effect::resolve,
    ApiType::BidLife => bid_life_effect::resolve,
    ApiType::Block => block_effect::resolve,
    ApiType::Bond => bond_effect::resolve,
    ApiType::ChooseCardName => choose_card_name_effect::resolve,
    ApiType::ChooseGeneric => choose_generic_effect::resolve,
    ApiType::ControlSpell => control_spell_effect::resolve,
    ApiType::DamagePrevent => damage_prevent_effect::resolve,
    ApiType::ExchangeLifeVariant => life_exchange_variant_effect::resolve,
    ApiType::ReplaceDamage => replace_damage_effect::resolve,
    ApiType::ReplaceMana => replace_mana_effect::resolve,
    ApiType::ReplaceCounter => replace_counter_effect::resolve,
    ApiType::ReplaceToken => replace_token_effect::resolve,
    ApiType::ReplaceSplitDamage => replace_split_damage_effect::resolve,
    ApiType::ExchangeTextBox => text_box_exchange_effect::resolve,
    ApiType::SwitchBlock => switch_block_effect::resolve,
    ApiType::ChangeCombatants => change_combatants_effect::resolve,
    ApiType::Mana => mana_effect::resolve,
    ApiType::ManaReflected => mana_reflected_effect::resolve,
    ApiType::Mill => mill_effect::resolve,
    ApiType::Scry => scry_effect::resolve,
    ApiType::Surveil => surveil_effect::resolve,
    ApiType::Dig => dig_effect::resolve,
    ApiType::DigMultiple => dig_multiple_effect::resolve,
    ApiType::RearrangeTopOfLibrary => rearrange_top_of_library_effect::resolve,
    ApiType::Reveal => reveal_effect::resolve,
    ApiType::RevealHand => reveal_hand_effect::resolve,
    ApiType::LookAt => look_at_effect::resolve,
    ApiType::Charm => charm_effect::resolve,
    ApiType::GenericChoice => charm_effect::resolve,
    ApiType::Plot => plot_effect::resolve,
    ApiType::PeekAndReveal => peek_and_reveal_effect::resolve,
    ApiType::SetState => set_state_effect::resolve,
    ApiType::Cleanup => cleanup_effect::resolve,
    ApiType::Counter => counter_effect::resolve,
    ApiType::GainControl => control_gain_effect::resolve,
    ApiType::Fight => fight_effect::resolve,
    ApiType::Discard => discard_effect::resolve,
    ApiType::Attach => attach_effect::resolve,
    ApiType::DestroyAll => destroy_all_effect::resolve,
    ApiType::DamageAll => damage_all_effect::resolve,
    ApiType::PumpAll => pump_all_effect::resolve,
    ApiType::TapAll => tap_all_effect::resolve,
    ApiType::TapOrUntapAll => tap_or_untap_all_effect::resolve,
    ApiType::UntapAll => untap_all_effect::resolve,
    ApiType::Tap => tap_effect::resolve,
    ApiType::TapOrUntap => tap_or_untap_effect::resolve,
    ApiType::Untap => untap_effect::resolve,
    ApiType::SetLife => life_set_effect::resolve,
    ApiType::ExchangeLife => life_exchange_effect::resolve,
    ApiType::WinsGame => game_win_effect::resolve,
    ApiType::LosesGame => game_loss_effect::resolve,
    ApiType::GameDrawn => game_draw_effect::resolve,
    ApiType::AddTurn => add_turn_effect::resolve,
    ApiType::ActivateAbility => activate_ability_effect::resolve,
    ApiType::Fog => fog_effect::resolve,
    ApiType::ReverseTurnOrder => reverse_turn_order_effect::resolve,
    ApiType::EndCombatPhase => end_combat_phase_effect::resolve,
    ApiType::EndTurn => end_turn_effect::resolve,
    ApiType::ExchangePower => power_exchange_effect::resolve,
    ApiType::BecomeMonarch => become_monarch_effect::resolve,
    ApiType::TakeInitiative => take_initiative_effect::resolve,
    ApiType::SkipTurn => skip_turn_effect::resolve,
    ApiType::SkipPhase => skip_phase_effect::resolve,
    ApiType::AddPhase => add_phase_effect::resolve,
    ApiType::Phases => phases_effect::resolve,
    ApiType::Regenerate => regenerate_effect::resolve,
    ApiType::Play => play_effect::resolve,
    ApiType::Animate => animate_effect::resolve,
    ApiType::AnimateAll => animate_all_effect::resolve,
    ApiType::Balance => balance_effect::resolve,
    ApiType::ChooseCard => choose_card_effect::resolve,
    ApiType::ChooseColor => choose_color_effect::resolve,
    ApiType::ChooseDirection => choose_direction_effect::resolve,
    ApiType::ChooseEvenOdd => choose_even_odd_effect::resolve,
    ApiType::Clone => clone_effect::resolve,
    ApiType::Connive => connive_effect::resolve,
    ApiType::GainControlVariant => control_gain_variant_effect::resolve,
    ApiType::RepeatEach => repeat_each_effect::resolve,
    ApiType::Shuffle => shuffle_effect::resolve,
    ApiType::PutCounterAll => counters_put_all_effect::resolve,
    ApiType::AddOrRemoveCounter => counters_put_or_remove_effect::resolve,
    ApiType::EachDamage => damage_each_effect::resolve,
    ApiType::Effect => effect_effect::resolve,
    ApiType::DelayedTrigger => delayed_trigger_effect::resolve,
    ApiType::DrainMana => drain_mana_effect::resolve,
    ApiType::RemoveFromCombat => remove_from_combat_effect::resolve,
    ApiType::Detain => detain_effect::resolve,
    ApiType::Goad => goad_effect::resolve,
    ApiType::ChoosePlayer => choose_player_effect::resolve,
    ApiType::ChooseSource => choose_source_effect::resolve,
    ApiType::ChooseType => choose_type_effect::resolve,
    ApiType::NameCard => name_card_effect::resolve,
    ApiType::ChooseNumber => choose_number_effect::resolve,
    ApiType::DigUntil => dig_until_effect::resolve,
    ApiType::FlipACoin => flip_coin_effect::resolve,
    ApiType::Explore => explore_effect::resolve,
    ApiType::RollDice => roll_dice_effect::resolve,
    ApiType::Protection => protect_effect::resolve,
    ApiType::ProtectionAll => protect_all_effect::resolve,
    ApiType::PreventDamage => prevent_damage_effect::resolve,
    ApiType::Proliferate => counters_proliferate_effect::resolve,
    ApiType::MoveCounter => move_counter_effect::resolve,
    ApiType::TimeTravel => time_travel_effect::resolve,
    ApiType::MustBlock => must_block_effect::resolve,
    ApiType::CopySpellAbility => copy_spell_ability_effect::resolve,
    ApiType::TwoPiles => two_piles_effect::resolve,
    ApiType::Encode => encode_effect::resolve,

    // ── Aliases: variants whose script names map via smart_value_of ────
    ApiType::ExchangeControlVariant => control_gain_variant_effect::resolve,
    ApiType::ExchangeZone => change_zone_effect::resolve,
    ApiType::Regeneration => regenerate_effect::resolve,

    // ── Niche/format-specific effects ─────────────────────────────────
    ApiType::Abandon => abandon_effect::resolve,
    ApiType::AdvanceCrank => advance_crank_effect::resolve,
    ApiType::Airbend => airbend_effect::resolve,
    ApiType::AlterAttribute => alter_attribute_effect::resolve,
    ApiType::AssembleContraption => assemble_contraption_effect::resolve,
    ApiType::AssignGroup => assign_group_effect::resolve,
    ApiType::BecomesBlocked => becomes_blocked_effect::resolve,
    ApiType::BlankLine => blank_line_effect::resolve,
    ApiType::Blight => blight_effect::resolve,
    ApiType::Camouflage => camouflage_effect::resolve,
    ApiType::ChangeSpeed => change_speed_effect::resolve,
    ApiType::ChaosEnsues => chaos_ensues_effect::resolve,
    ApiType::ChooseSector => choose_sector_effect::resolve,
    ApiType::ClaimThePrize => claim_the_prize_effect::resolve,
    ApiType::DamageResolve => damage_resolve_effect::resolve,
    ApiType::Debuff => debuff_effect::resolve,
    ApiType::Draft => draft_effect::resolve,
    ApiType::Earthbend => earthbend_effect::resolve,
    ApiType::Endure => endure_effect::resolve,
    ApiType::GainOwnership => ownership_gain_effect::resolve,
    ApiType::Intensify => intensify_effect::resolve,
    ApiType::LosePerpetual => lose_perpetual_effect::resolve,
    ApiType::MakeCard => make_card_effect::resolve,
    ApiType::MultiplePiles => multiple_piles_effect::resolve,
    ApiType::Mutate => mutate_effect::resolve,
    ApiType::OpenAttraction => open_attraction_effect::resolve,
    ApiType::PermanentCreature => permanent_creature_effect::resolve,
    ApiType::PermanentNoncreature => permanent_noncreature_effect::resolve,
    ApiType::Planeswalk => planeswalk_effect::resolve,
    ApiType::Radiation => radiation_effect::resolve,
    ApiType::InternalRadiation => internal_radiation_effect::resolve,
    ApiType::ZoneExchange => zone_exchange_effect::resolve,
    ApiType::RemoveFromGame => remove_from_game_effect::resolve,
    ApiType::RemoveFromMatch => remove_from_match_effect::resolve,
    ApiType::RestartGame => restart_game_effect::resolve,
    ApiType::RollPlanarDice => roll_planar_dice_effect::resolve,
    ApiType::RunChaos => run_chaos_effect::resolve,
    ApiType::SetInMotion => set_in_motion_effect::resolve,
    ApiType::Subgame => subgame_effect::resolve,
    ApiType::UnlockDoor => unlock_door_effect::resolve,
    ApiType::ChangeZoneResolve => change_zone_resolve_effect::resolve,
    ApiType::PlayLandVariant => play_land_variant_effect::resolve,
}

/// Everything an effect needs to resolve.
pub struct EffectContext<'a> {
    pub game: &'a mut GameState,
    pub combat: Option<&'a mut crate::combat::CombatState>,
    pub agents: &'a mut [Box<dyn PlayerAgent>],
    pub trigger_handler: &'a mut TriggerHandler,
    pub token_templates: &'a HashMap<String, Card>,
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

fn choose_defender(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    controller: PlayerId,
    defenders: &[DefenderId],
) -> Option<DefenderId> {
    if defenders.is_empty() {
        return None;
    }
    if defenders.len() == 1 {
        return Some(defenders[0]);
    }

    let valid_players: Vec<PlayerId> = defenders.iter().filter_map(|d| d.as_player()).collect();
    let valid_cards: Vec<CardId> = defenders
        .iter()
        .filter_map(|d| match d {
            DefenderId::Permanent(cid) => Some(*cid),
            DefenderId::Player(_) => None,
        })
        .collect();
    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
    Some(
        match ctx.agents[controller.index()].choose_target_any(
            controller,
            &valid_players,
            &valid_cards,
            Some(sa),
        ) {
            crate::agent::TargetChoice::Player(pid) => DefenderId::Player(pid),
            crate::agent::TargetChoice::Card(cid) => DefenderId::Permanent(cid),
            _ => defenders[0],
        },
    )
}

fn resolve_attack_defenders(
    ctx: &EffectContext,
    sa: &SpellAbility,
    card_id: CardId,
    attacking_param: &str,
) -> Vec<DefenderId> {
    let controller = ctx.game.card(card_id).controller;
    let possible = crate::combat::get_possible_defenders(ctx.game, controller);
    if attacking_param.eq_ignore_ascii_case("True") {
        return possible;
    }

    let mut defenders: Vec<DefenderId> =
        crate::ability::ability_utils::resolve_defined_players_with_sa(
            attacking_param,
            sa,
            sa.activating_player,
            ctx.game,
        )
        .into_iter()
        .map(DefenderId::Player)
        .collect();

    if defenders.is_empty() {
        defenders.extend(
            sa.trigger_objects
                .get("Attacked")
                .into_iter()
                .flat_map(|value| value.split(','))
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .map(CardId)
                .map(DefenderId::Permanent),
        );
    }

    defenders.retain(|defender| possible.contains(defender));
    defenders
}

pub(crate) fn emit_targeting_triggers(
    ctx: &mut EffectContext,
    card_id: CardId,
    trigger_sa: &SpellAbility,
) {
    let controller = trigger_sa.activating_player;
    if let Some(target_id) = trigger_sa.target_chosen.target_card {
        let first_time = !ctx.game.card(target_id).has_become_target_this_turn();
        ctx.game.card_mut(target_id).add_target_from_this_turn();
        let params = RunParams {
            card: Some(target_id),
            target_card: Some(target_id),
            cards: Some(vec![target_id]),
            cause_player: Some(controller),
            cause_card: Some(card_id),
            source_sa: Some(trigger_sa.clone()),
            first_time: Some(first_time),
            ..Default::default()
        };
        ctx.trigger_handler
            .run_trigger(TriggerType::BecomesTarget, params.clone(), false);
        ctx.trigger_handler
            .run_trigger(TriggerType::BecomesTargetOnce, params, false);
    } else if let Some(target_player) = trigger_sa.target_chosen.target_player {
        let params = RunParams {
            player: Some(target_player),
            target_player: Some(target_player),
            cause_player: Some(controller),
            cause_card: Some(card_id),
            source_sa: Some(trigger_sa.clone()),
            ..Default::default()
        };
        ctx.trigger_handler
            .run_trigger(TriggerType::BecomesTarget, params.clone(), false);
        ctx.trigger_handler
            .run_trigger(TriggerType::BecomesTargetOnce, params, false);
    }
}

pub(crate) fn add_to_combat(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    card_id: CardId,
    attacking_param: &str,
) -> bool {
    if !ctx.game.turn.is_combat() || !ctx.game.card(card_id).is_creature() {
        return false;
    }

    let controller = ctx.game.card(card_id).controller;

    let Some(attacking) = sa.params.get(attacking_param) else {
        return false;
    };
    let defenders = resolve_attack_defenders(ctx, sa, card_id, attacking);
    let Some(defender) = choose_defender(ctx, sa, controller, &defenders) else {
        return false;
    };

    let Some(combat) = ctx.combat.as_deref_mut() else {
        return false;
    };
    let Some(attacking_player) = combat.attacking_player else {
        return false;
    };
    if attacking_player != controller {
        return false;
    }

    if combat
        .attackers
        .iter()
        .any(|&(attacker, current)| attacker == card_id && current == defender)
    {
        return false;
    }

    combat.remove_from_combat(card_id, ctx.game);
    combat.add_attacker(card_id, defender);

    let defending_player = defender.controlling_player(ctx.game);
    let tracked_defender = match defender {
        DefenderId::Player(pid) => crate::card::card_damage_history::TrackedEntity::Player(pid),
        DefenderId::Permanent(cid) => crate::card::card_damage_history::TrackedEntity::Card(cid),
    };
    let num_other_attackers = combat.attackers.len().saturating_sub(1) as i32;
    let defender_is_battle = matches!(
        defender,
        DefenderId::Permanent(cid) if ctx.game.card(cid).type_line.core_types.contains(&CoreType::Battle)
    );

    let card = ctx.game.card_mut(card_id);
    card.set_attacking_player(defending_player);
    card.mark_attacked_this_turn();
    card.damage_history.set_creature_attacked_this_combat(
        Some(tracked_defender),
        num_other_attackers,
        defender_is_battle,
    );
    true
}

/// Check if a conditional gate on this SA is satisfied.
/// Handles `Condition$ Kicked` (simple gate) and `ConditionCheckSVar$ Kicked` (SVar-based gate).
/// Mirrors Java's `SpellAbility.checkConditions()`.
fn check_condition(sa: &SpellAbility) -> bool {
    // Check Condition$ Kicked (most common pattern: simple kicked gate)
    if let Some(cond) = sa.params.get(keys::CONDITION) {
        if cond == "Kicked" {
            return sa.kicked;
        }
    }
    // Check ConditionCheckSVar$ Kicked (SVar-based kicked gate)
    if let Some(cond) = sa.params.get(keys::CONDITION_CHECK_SVAR) {
        if cond == "Kicked" || cond == "X:Kicked" {
            return sa.kicked;
        }
    }
    true
}

/// Check ConditionPresent$ / ConditionZone$ / ConditionCompare$ against game state.
/// Returns true if the condition is met (or if no condition params exist).
///
/// When `ConditionDefined$` is present, check the defined cards instead of
/// scanning a zone.  Mirrors Java's `SpellAbilityCondition.checkConditions()`.
fn check_condition_present(
    game: &GameState,
    sa: &SpellAbility,
    player: PlayerId,
    source_id: CardId,
) -> bool {
    let condition = match sa.params.get_cloned(keys::CONDITION_PRESENT) {
        Some(c) => c,
        None => return true, // No condition — always passes
    };

    // Parse condition alternatives (comma-separated)
    let alternatives: Vec<&str> = condition.split(',').map(|s| s.trim()).collect();

    // ── ConditionDefined$ — check specific defined cards, not a zone ──
    if let Some(cond_defined) = sa.params.get(keys::CONDITION_DEFINED) {
        let defined_cards: Vec<CardId> = match cond_defined {
            "Targeted" => sa.target_chosen.target_card.into_iter().collect(),
            "Self" => sa.source.into_iter().collect(),
            "Remembered" => sa
                .source
                .map(|sid| game.card(sid).remembered_cards.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        };

        let count = defined_cards
            .iter()
            .filter(|&&cid| matches_condition_filter(game, cid, source_id, player, &alternatives))
            .count() as i32;

        return if let Some(compare) = sa.params.get(keys::CONDITION_COMPARE) {
            compare_expr(count, compare)
        } else {
            count > 0
        };
    }

    let zone_str = sa.params.get(keys::CONDITION_ZONE).unwrap_or("Battlefield");

    let zone = match zone_str.to_ascii_lowercase().as_str() {
        "graveyard" => ZoneType::Graveyard,
        "hand" => ZoneType::Hand,
        "exile" => ZoneType::Exile,
        "library" => ZoneType::Library,
        _ => ZoneType::Battlefield,
    };

    // Count matching cards in zone
    let cards = game.cards_in_zone(zone, player);
    let count = cards
        .iter()
        .filter(|&&cid| matches_condition_filter(game, cid, source_id, player, &alternatives))
        .count() as i32;

    // Check ConditionCompare$ (e.g. "GE2", "EQ0")
    if let Some(compare) = sa.params.get(keys::CONDITION_COMPARE) {
        compare_expr(count, compare)
    } else {
        count > 0
    }
}

/// Check if a card matches a condition filter expression.
/// Handles type matching + qualifier checks (YouCtrl, OppCtrl, ChosenCtrl, etc.).
fn matches_condition_filter(
    game: &GameState,
    cid: CardId,
    source_id: CardId,
    player: PlayerId,
    alternatives: &[&str],
) -> bool {
    if cid == source_id {
        return false; // Don't count self
    }
    let card = game.card(cid);
    let source = game.card(source_id);
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
                "chosenctrl" => {
                    // Card must be controlled by the secretly chosen player
                    source
                        .chosen_player
                        .map_or(false, |chosen| card.controller == chosen)
                }
                _ => true,
            }
        } else {
            true
        }
    })
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
    let repeat_count = if let Some(repeat_val) = sa.params.get(keys::REPEAT) {
        match repeat_val {
            "KickerNum" => sa.kick_count as i32,
            _ => 1,
        }
    } else {
        1
    };

    for _ in 0..repeat_count {
        if let Some(unless_cost) = sa
            .params
            .get(keys::UNLESS_COST)
            .map(|s: &str| s.trim())
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

    let is_switched = sa.params.has(keys::UNLESS_SWITCHED);
    if already_paid == is_switched {
        resolve_effect_once(ctx, sa);
    }
}

fn resolve_unless_payers(sa: &SpellAbility, game: &GameState) -> Vec<PlayerId> {
    let pays = sa
        .params
        .get(keys::UNLESS_PAYER)
        .unwrap_or("TargetedController");
    if pays.eq_ignore_ascii_case("TargetedController") {
        if let Some(pid) = sa.target_chosen.target_player {
            vec![pid]
        } else {
            vec![game.opponent_of(sa.activating_player)]
        }
    } else {
        helpers::resolve_defined_players(pays, sa.activating_player, game)
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
            | CostPart::Discard { .. }
            | CostPart::Sacrifice { .. }
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
                // Java: CostDiscard.visit() → confirm(cost, true)
                CostPart::Discard { .. } => true,
                // Java: CostSacrifice.visit() → confirm(cost, true)
                CostPart::Sacrifice { .. } => true,
                // Java: CostPayEnergy, CostPayShards, CostPutCounter, CostPartMana → no confirm
                _ => false,
            };
            if should_ask {
                let card_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());
                let api = sa.api;
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
            CostPart::Discard {
                amount,
                type_filter,
            } => {
                // UnlessCost discard: pick cards from hand and discard them.
                for _ in 0..*amount {
                    let valid: Vec<CardId> = ctx
                        .game
                        .cards_in_zone(ZoneType::Hand, payer)
                        .to_vec()
                        .into_iter()
                        .filter(|&cid| {
                            if type_filter == "Card" || type_filter.is_empty() {
                                true
                            } else {
                                crate::ability::effects::helpers::matches_change_type(
                                    ctx.game.card(cid),
                                    type_filter,
                                    &[],
                                )
                            }
                        })
                        .collect();
                    if valid.is_empty() {
                        return false;
                    }
                    let chosen = ctx.agents[payer.index()].choose_discard(payer, &valid, 1);
                    if let Some(&cid) = chosen.first() {
                        helpers::discard_with_madness_replacement(
                            ctx.game,
                            ctx.trigger_handler,
                            cid,
                            payer,
                        );
                    }
                }
            }
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                for _ in 0..*amount {
                    let valid = crate::cost::get_sacrifice_targets(ctx.game, payer, type_filter);
                    if valid.is_empty() {
                        return false;
                    }
                    if let Some(chosen) =
                        ctx.agents[payer.index()].choose_sacrifice(payer, &valid, None)
                    {
                        let owner = ctx.game.card(chosen).owner;
                        ctx.trigger_handler.run_trigger(
                            TriggerType::Sacrificed,
                            RunParams {
                                card: Some(chosen),
                                player: Some(payer),
                                ..Default::default()
                            },
                            false,
                        );
                        ctx.game.move_card(chosen, ZoneType::Graveyard, owner);
                        emit_zone_trigger(
                            &mut ctx.trigger_handler,
                            chosen,
                            ZoneType::Battlefield,
                            ZoneType::Graveyard,
                        );
                    }
                }
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
            let api = sa.api;
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
        CostPart::FlipCoin(_) => "FlipCoin",
        _ => "Cost",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_num_dmg_test() {
        assert_eq!(
            helpers::parse_num_dmg(
                "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ test"
            ),
            3
        );
    }

    #[test]
    fn parse_param_test() {
        assert_eq!(
            helpers::parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumAtt$ "),
            Some(3)
        );
        assert_eq!(
            helpers::parse_param("SP$ Pump | NumAtt$ 3 | NumDef$ 3", "NumDef$ "),
            Some(3)
        );
        assert_eq!(
            helpers::parse_param("SP$ Draw | NumCards$ 2", "NumCards$ "),
            Some(2)
        );
    }
}
