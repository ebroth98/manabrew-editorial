use forge_foundation::ZoneType;

use super::EffectContext;
use crate::agent::GameEntity;
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::replacement::handler::{apply_replacements, ReplacementEvent};
use crate::spellability::SpellAbility;

/// `SP$ Proliferate` — choose any number of permanents and/or players that
/// have counters on them, then add one counter of each kind already there.
///
/// Mirrors Java's `CountersProliferateEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ Proliferate
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Build unified candidate list: players first, then permanents (matches Java order).
    // Java: list.addAll(game.getPlayers().filter(PlayerPredicates.hasCounters()));
    //       list.addAll(CardLists.filter(game.getCardsIn(ZoneType.Battlefield), CardPredicates.hasCounters()));
    let player_ids = ctx.game.player_order.clone();
    let mut candidates: Vec<GameEntity> = Vec::new();

    // Players with counters (poison, energy, etc.)
    for &pid in &player_ids {
        let p = ctx.game.player(pid);
        if p.poison_counters > 0 || p.energy_counters > 0 {
            candidates.push(GameEntity::Player(pid));
        }
    }

    // Battlefield permanents with counters
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if !ctx.game.card(cid).counters.is_empty()
                && ctx.game.card(cid).counters.values().any(|&v| v > 0)
            {
                candidates.push(GameEntity::Card(cid));
            }
        }
    }

    if candidates.is_empty() {
        return;
    }

    // Player chooses which entities to proliferate (0..all)
    let chosen = ctx.agents[controller.index()].choose_entities_for_effect(
        controller,
        &candidates,
        0,
        candidates.len(),
    );

    // Add one counter of each type already present on chosen entities
    for entity in chosen {
        match entity {
            GameEntity::Player(pid) => {
                proliferate_player(ctx, pid);
            }
            GameEntity::Card(cid) => {
                proliferate_card(ctx, cid);
            }
        }
    }
}

/// Add one of each counter type already on a player.
fn proliferate_player(ctx: &mut EffectContext, pid: crate::ids::PlayerId) {
    let p = ctx.game.player(pid);
    if p.poison_counters > 0 {
        ctx.game.player_mut(pid).poison_counters += 1;
    }
    let p = ctx.game.player(pid);
    if p.energy_counters > 0 {
        ctx.game.player_mut(pid).energy_counters += 1;
    }
}

/// Add one of each counter type already on a permanent.
fn proliferate_card(ctx: &mut EffectContext, cid: CardId) {
    if ctx.game.card(cid).zone != ZoneType::Battlefield {
        return;
    }
    // Snapshot existing counter types
    let counter_types: Vec<CounterType> = ctx
        .game
        .card(cid)
        .counters
        .iter()
        .filter(|(_, &count)| count > 0)
        .map(|(ct, _)| ct.clone())
        .collect();

    for ct in &counter_types {
        if crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
            &ctx.game.cards,
            ctx.game.card(cid),
            ct,
        ) {
            continue;
        }
        if let Some(max) = crate::staticability::static_ability_max_counter::max_counter(
            &ctx.game.cards,
            ctx.game.card(cid),
            ct,
        ) {
            if ctx.game.card(cid).counter_count(ct) >= max {
                continue;
            }
        }
        // Run AddCounter replacement effects.
        let mut add_event = ReplacementEvent::AddCounter {
            target: cid,
            counter_type: ct.clone(),
            count: 1,
        };
        apply_replacements(ctx.game, &mut add_event);
        let final_count = if let ReplacementEvent::AddCounter { count, .. } = add_event {
            count
        } else {
            1
        };
        ctx.game.card_mut(cid).add_counter(ct, final_count);
        ctx.trigger_handler.run_trigger(
            TriggerType::CounterAdded,
            RunParams {
                card: Some(cid),
                counter_type: Some(format!("{:?}", ct)),
                counter_amount: Some(1),
                ..Default::default()
            },
            false,
        );
    }
}
