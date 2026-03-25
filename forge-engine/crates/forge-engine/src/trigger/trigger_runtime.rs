use crate::agent::{notify_all_agents, GameLogEvent, PlayerAgent};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::CardId;
use crate::mana::ManaPool;
use forge_foundation::ZoneType;

use super::handler::{DelayedTrigger, PendingTrigger, TriggerHandler};

#[derive(Debug, Clone)]
pub struct TriggerPushLog {
    pub source_name: String,
    pub player_name: String,
    pub optional: bool,
    pub trigger_api: String,
}

/// Processes queued triggers and pushes resolved pending trigger abilities onto the stack.
/// This is the trigger-module owned equivalent of Java TriggerHandler stack handoff.
pub fn process_waiting_triggers(
    trigger_handler: &mut TriggerHandler,
    mana_pools: &[ManaPool],
    game: &mut GameState,
    agents: &mut [Box<dyn PlayerAgent>],
) -> Vec<TriggerPushLog> {
    let pending = trigger_handler.run_waiting_triggers(game);
    process_pending_triggers(trigger_handler, mana_pools, game, agents, pending)
}

pub fn process_pending_triggers(
    trigger_handler: &mut TriggerHandler,
    mana_pools: &[ManaPool],
    game: &mut GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    pending: Vec<PendingTrigger>,
) -> Vec<TriggerPushLog> {
    let mut push_logs = Vec::new();
    for mut pt in pending {
        let source_name = pt
            .entry
            .spell_ability
            .source
            .and_then(|id| game.cards.get(id.index()).map(|c| c.card_name.clone()))
            .unwrap_or_else(|| "Triggered ability".to_string());
        let player_name = game
            .player(pt.entry.spell_ability.activating_player)
            .name
            .clone();

        // Match Java deterministic flow: charm choice before target setup.
        if pt.entry.spell_ability.api == Some(crate::ability::api_type::ApiType::Charm) {
            if !crate::ability::effects::charm_effect::make_choices_precast(
                game,
                agents,
                &mut pt.entry.spell_ability,
            ) {
                continue;
            }
        }

        if !pt
            .entry
            .spell_ability
            .setup_targets(game, agents, mana_pools)
        {
            continue;
        }

        if pt.optional {
            pt.entry.optional_trigger_decider = Some(pt.decider);
            pt.entry.optional_trigger_description = Some(pt.description.clone());
            pt.entry.optional_trigger_source_name = Some(source_name.clone());
        }

        // Java parity: mark trigger activation when it is placed on the stack.
        if let Some(source_id) = pt.entry.spell_ability.trigger_source {
            if let Some(trig_idx) = pt.entry.spell_ability.trigger_index {
                if let Some(trigger) = game
                    .cards
                    .get(source_id.index())
                    .and_then(|c| c.triggers.get(trig_idx))
                    .cloned()
                {
                    trigger.trigger_run(game, source_id);
                }
            }
        }

        let trigger_mode = pt
            .entry
            .spell_ability
            .trigger_source
            .and_then(|source_id| {
                pt.entry.spell_ability.trigger_index.and_then(|idx| {
                    game.cards
                        .get(source_id.index())
                        .and_then(|c| c.triggers.get(idx))
                        .map(|t| trigger_mode_name(&t.mode))
                })
            })
            .unwrap_or_else(|| "DelayedOrUnknown".to_string());
        let trigger_api = pt
            .entry
            .spell_ability
            .api
            .map(|a| a.name().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let trigger_msg = if pt.description.is_empty() {
            format!(
                "Trigger fired: mode={} | api={} | source={}",
                trigger_mode, trigger_api, source_name
            )
        } else {
            format!(
                "Trigger fired: mode={} | api={} | source={} | {}",
                trigger_mode, trigger_api, source_name, pt.description
            )
        };
        let mut event =
            GameLogEvent::stack(trigger_msg).with_player(pt.entry.spell_ability.activating_player);
        if let Some(source_id) = pt.entry.spell_ability.source {
            event = event.with_source_card(source_id);
        }
        if let Some(target_id) = pt.entry.spell_ability.target_chosen.target_card {
            event = event.with_target_card(target_id);
        }
        notify_all_agents(agents, event);

        let is_optional = pt.optional;
        let pushed_entry = pt.entry.clone();
        game.stack.push(pt.entry);
        if pushed_entry.spell_ability.is_trigger {
            let source_card = pushed_entry.spell_ability.source;
            trigger_handler.run_trigger(
                TriggerType::SpellAbilityCast,
                RunParams {
                    card: source_card,
                    spell_card: source_card,
                    player: Some(pushed_entry.spell_ability.activating_player),
                    activator: Some(pushed_entry.spell_ability.activating_player),
                    spell_controller: Some(pushed_entry.spell_ability.activating_player),
                    spell_ability: Some(pushed_entry.spell_ability.clone()),
                    source_sa: Some(pushed_entry.spell_ability.clone()),
                    cause: Some(pushed_entry.spell_ability.clone()),
                    cause_card: source_card,
                    ..Default::default()
                },
                true,
            );
            let stored_mode = pushed_entry
                .spell_ability
                .trigger_objects
                .get("AbilityTriggeredMode")
                .cloned()
                .unwrap_or_else(|| trigger_mode.clone());
            let stored_destinations = pushed_entry
                .spell_ability
                .trigger_objects
                .get("AbilityTriggeredDestinations")
                .cloned();
            let stored_cause_cards = pushed_entry
                .spell_ability
                .trigger_objects
                .get("AbilityTriggeredCauseCards")
                .map(|csv| {
                    csv.split(',')
                        .filter_map(|part| part.trim().parse::<u32>().ok())
                        .map(crate::ids::CardId)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| source_card.map(|c| vec![c]).unwrap_or_default());
            trigger_handler.run_trigger(
                TriggerType::AbilityTriggered,
                RunParams {
                    spell_ability: Some(pushed_entry.spell_ability.clone()),
                    source_sa: Some(pushed_entry.spell_ability.clone()),
                    cause_card: source_card,
                    cards: if stored_cause_cards.is_empty() {
                        None
                    } else {
                        Some(stored_cause_cards)
                    },
                    mode: Some(stored_mode),
                    destination: stored_destinations
                        .as_deref()
                        .and_then(|csv| csv.split(',').next())
                        .and_then(parse_zone_name),
                    destinations: stored_destinations,
                    ..Default::default()
                },
                false,
            );
        }
        push_logs.push(TriggerPushLog {
            source_name,
            player_name,
            optional: is_optional,
            trigger_api,
        });
    }
    push_logs
}

fn trigger_mode_name(mode: &crate::trigger::TriggerMode) -> String {
    let dbg = format!("{:?}", mode);
    dbg.split(|c: char| c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

fn parse_zone_name(name: &str) -> Option<ZoneType> {
    match name.trim() {
        "Ante" => Some(ZoneType::Ante),
        "Battlefield" => Some(ZoneType::Battlefield),
        "Command" => Some(ZoneType::Command),
        "Exile" => Some(ZoneType::Exile),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" => Some(ZoneType::Library),
        "Stack" => Some(ZoneType::Stack),
        _ => None,
    }
}

// Compatibility wrappers so trigger-owned API surface maps to Java naming.
pub fn register_delayed_trigger(handler: &mut TriggerHandler, delayed: DelayedTrigger) {
    handler.register_delayed_trigger(delayed);
}

pub fn run_trigger(handler: &mut TriggerHandler, mode: TriggerType, params: RunParams, hold: bool) {
    handler.run_trigger(mode, params, hold);
}

pub fn run_waiting_triggers(handler: &mut TriggerHandler, game: &GameState) -> Vec<PendingTrigger> {
    handler.run_waiting_triggers(game)
}

pub fn reset_active_triggers(handler: &mut TriggerHandler, game: &GameState) {
    handler.reset_active_triggers(game);
}

pub fn register_active_trigger(handler: &mut TriggerHandler, game: &GameState, card_id: CardId) {
    handler.register_active_trigger(game, card_id);
}

pub fn clear_active_triggers(handler: &mut TriggerHandler, card_id: CardId) {
    handler.unregister_active_triggers(card_id);
}

pub fn suppress_mode(handler: &mut TriggerHandler, mode: TriggerType) {
    handler.suppress_mode(mode);
}

pub fn clear_suppression(handler: &mut TriggerHandler, mode: TriggerType) {
    handler.clear_suppression(mode);
}

pub fn clear_waiting_triggers(_handler: &mut TriggerHandler) {
    _handler.clear_waiting_triggers();
}
