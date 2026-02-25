use forge_foundation::ZoneType;

use super::EffectContext;
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
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

    // Collect all battlefield permanents with at least one counter
    let player_ids = ctx.game.player_order.clone();
    let mut candidates: Vec<CardId> = Vec::new();

    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if !ctx.game.card(cid).counters.is_empty()
                && ctx.game.card(cid).counters.values().any(|&v| v > 0)
            {
                candidates.push(cid);
            }
        }
    }

    if candidates.is_empty() {
        return;
    }

    // Player chooses which permanents to proliferate (0..all)
    let chosen = ctx.agents[controller.index()].choose_cards_for_effect(
        controller,
        &candidates,
        0,
        candidates.len(),
    );

    // Add one counter of each type already present on chosen permanents
    for cid in chosen {
        if ctx.game.card(cid).zone != ZoneType::Battlefield {
            continue;
        }
        // Snapshot existing counter types
        let counter_types: Vec<CounterType> = ctx.game.card(cid)
            .counters
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(&ct, _)| ct)
            .collect();

        for ct in counter_types {
            ctx.game.card_mut(cid).add_counter(ct, 1);
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
}
