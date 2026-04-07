//! Replacement logic for `Event$ Moved`.
//!
//! Mirrors Java `ReplaceMoved.java` in `forge/game/replacement/`.

use std::collections::HashMap;

use forge_foundation::ZoneType;

use crate::ability::effects::{self, EffectContext};
use crate::agent::{PassAgent, PlayerAgent};
use crate::card::Card;
use crate::game::GameState;
use crate::game_rng::ThreadRngAdapter;
use crate::ids::CardId;
use crate::mana::ManaPool;
use crate::parsing::keys;
use crate::spellability::build_spell_ability;
use crate::trigger::TriggerHandler;

use super::replacement_effect::{matches_valid_card, zone_matches, ReplacementEffect};
use super::replacement_handler::{ReplacementEvent, ReplacementRuntime};
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceMoved.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::Moved {
        return false;
    }
    let (moving_id, origin, destination) = match event {
        ReplacementEvent::Moved {
            card,
            origin,
            destination,
        } => (*card, *origin, *destination),
        _ => return false,
    };
    if let Some(dest) = effect.params.get(keys::DESTINATION) {
        if !zone_matches(dest, destination) {
            return false;
        }
    }
    if let Some(orig) = effect.params.get(keys::ORIGIN) {
        if !zone_matches(orig, origin) {
            return false;
        }
    }
    let moving_card = &game.cards[moving_id.index()];
    if let Some(valid) = effect.params.get(keys::VALID_CARD) {
        if !matches_valid_card(valid, moving_card, source_card) {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for Moved.
pub fn execute(
    effect: &ReplacementEffect,
    event: &mut ReplacementEvent,
    game: &mut GameState,
    source_card_id: CardId,
    mut agents: Option<&mut [Box<dyn PlayerAgent>]>,
    mut runtime: Option<&mut ReplacementRuntime<'_>>,
) -> ReplacementResult {
    let destination = match event {
        ReplacementEvent::Moved { destination, .. } => destination,
        _ => return ReplacementResult::NotReplaced,
    };
    // Check NewDestination$ first (explicit redirect), then ReplaceWith$ (common alias).
    // Rest in Peace uses "ReplaceWith$ Exile", while other cards use "NewDestination$ Exile".
    let redirect = effect
        .params
        .get(keys::NEW_DESTINATION)
        .or_else(|| effect.params.get(keys::REPLACE_WITH));

    if let Some(new_dest) = redirect {
        let new_zone = match new_dest.trim() {
            "Exile" => Some(ZoneType::Exile),
            "Graveyard" => Some(ZoneType::Graveyard),
            "Hand" => Some(ZoneType::Hand),
            "Library" => Some(ZoneType::Library),
            "Battlefield" => Some(ZoneType::Battlefield),
            "Command" => Some(ZoneType::Command),
            _ => None,
        };
        if let Some(z) = new_zone {
            if let ReplacementEvent::Moved { destination, .. } = event {
                *destination = z;
            }
            return ReplacementResult::Updated;
        }
    }
    // If the redirect value wasn't a zone name, try executing it as an SVar spell ability.
    if let Some(replace_with_key) = effect.params.get(keys::REPLACE_WITH) {
        let succeeded = execute_replace_with(
            replace_with_key,
            game,
            source_card_id,
            event,
            agents,
            runtime,
        );
        if !succeeded {
            return ReplacementResult::NotReplaced;
        }
    }
    ReplacementResult::Replaced
}

fn execute_replace_with(
    replace_with: &str,
    game: &mut GameState,
    source_card_id: CardId,
    event: &ReplacementEvent,
    agents: Option<&mut [Box<dyn PlayerAgent>]>,
    mut runtime: Option<&mut ReplacementRuntime<'_>>,
) -> bool {
    let Some(raw) = game.card(source_card_id).svars.get(replace_with).cloned() else {
        return false;
    };
    let controller = game.card(source_card_id).controller;
    let mut sa = build_spell_ability(game, source_card_id, &raw, controller);
    set_replacing_objects_for_moved(event, &mut sa);

    let mut local_agents_storage: Option<Vec<Box<dyn PlayerAgent>>> = None;
    let agents: &mut [Box<dyn PlayerAgent>] = if let Some(agents) = agents {
        agents
    } else {
        local_agents_storage = Some(
            (0..game.players.len())
                .map(|_| Box::new(PassAgent) as Box<dyn PlayerAgent>)
                .collect(),
        );
        local_agents_storage.as_mut().unwrap().as_mut_slice()
    };

    let mut local_mana_pools: Vec<ManaPool> =
        (0..game.players.len()).map(|_| ManaPool::new()).collect();
    let mana_pools_for_targets: &[ManaPool] = if let Some(rt) = runtime.as_ref() {
        rt.mana_pools.as_slice()
    } else {
        local_mana_pools.as_slice()
    };
    if sa.uses_targeting() && !sa.setup_targets(game, agents, mana_pools_for_targets) {
        return false;
    }

    let mut local_trigger_handler = TriggerHandler::new();
    let local_token_templates: HashMap<String, Card> = HashMap::new();
    let mut local_rng = ThreadRngAdapter;

    let mut parent_target_card: Option<CardId> = None;
    let mut parent_target_player = None;
    let mut current_sa: Option<&crate::spellability::SpellAbility> = Some(&sa);
    while let Some(cur) = current_sa {
        let mut sa_with_ctx;
        let sa_ref = if parent_target_player.is_some() && cur.target_chosen.target_player.is_none()
        {
            sa_with_ctx = cur.clone();
            sa_with_ctx.target_chosen.target_player = parent_target_player;
            &sa_with_ctx
        } else {
            cur
        };

        let (trigger_handler_ref, token_templates_ref, mana_pools_ref, rng_ref): (
            &mut TriggerHandler,
            &HashMap<String, Card>,
            &mut Vec<ManaPool>,
            &mut dyn crate::game_rng::GameRng,
        ) = if let Some(rt) = runtime.as_deref_mut() {
            (
                rt.trigger_handler,
                rt.token_templates,
                rt.mana_pools,
                rt.rng,
            )
        } else {
            (
                &mut local_trigger_handler,
                &local_token_templates,
                &mut local_mana_pools,
                &mut local_rng,
            )
        };

        let mut ctx = EffectContext {
            game,
            combat: None,
            agents,
            trigger_handler: trigger_handler_ref,
            token_templates: token_templates_ref,
            mana_pools: mana_pools_ref,
            parent_target_card,
            rng: rng_ref,
        };
        effects::resolve_effect(&mut ctx, sa_ref);
        parent_target_card = sa_ref.target_chosen.target_card;
        parent_target_player = sa_ref.target_chosen.target_player;
        current_sa = cur.get_sub_ability();
    }
    true
}

fn set_replacing_objects_for_moved(
    event: &ReplacementEvent,
    sa: &mut crate::spellability::SpellAbility,
) {
    let ReplacementEvent::Moved { card, .. } = event else {
        return;
    };
    let card_csv = card.0.to_string();
    let mut current = Some(sa);
    while let Some(node) = current {
        node.add_triggering_object("Card", &card_csv);
        node.add_triggering_object("ReplacedCard", &card_csv);
        current = node.get_sub_ability_mut();
    }
}
