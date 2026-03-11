use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// `SP$ Explore` — target creature explores.
///
/// Mirrors Java's `ExploreEffect.java`.
/// Explore: Reveal the top card of your library. If it's a land, put it into your hand.
/// Otherwise, put a +1/+1 counter on this creature, then you may put the card into
/// your graveyard.
///
/// # Card script examples
/// ```text
/// A:SP$ Explore | Defined$ Self
/// A:SP$ Explore | Defined$ Targeted | Num$ 2
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Determine the exploring creature
    let explorer = sa.target_chosen.target_card.or_else(|| {
        match sa.params.get("Defined").map(|s| s.as_str()) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => sa.source,
        }
    });

    let explorer_id = match explorer {
        Some(id) if ctx.game.card(id).zone == ZoneType::Battlefield => id,
        _ => return,
    };

    // Parse Num parameter for multiple explores (e.g. Jadelight Ranger explores twice).
    // Mirrors Java's `AbilityUtils.calculateAmount(host, sa.getParamOrDefault("Num", "1"), sa)`.
    let amount: i32 = sa
        .params
        .get("Num")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    for _ in 0..amount {
        // Re-check explorer is still on battlefield (may have been removed by a trigger)
        if ctx.game.card(explorer_id).zone != ZoneType::Battlefield {
            return;
        }

        // Check if library has cards
        let lib = ctx.game.cards_in_zone(ZoneType::Library, controller);
        if lib.is_empty() {
            // Explorer still gets the +1/+1 counter per rules
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                &ctx.game.cards,
                ctx.game.card(explorer_id),
                &CounterType::P1P1,
            ) {
                ctx.game
                    .card_mut(explorer_id)
                    .add_counter(&CounterType::P1P1, 1);
            }
            ctx.trigger_handler.run_trigger(
                TriggerType::CounterAdded,
                RunParams {
                    card: Some(explorer_id),
                    counter_type: Some("P1P1".to_string()),
                    counter_amount: Some(1),
                    ..Default::default()
                },
                false,
            );
            continue;
        }

        // Reveal top card
        let top_card = *lib.last().unwrap();

        // Let UI agents build card info
        ctx.agents[controller.index()].on_library_peek(ctx.game, &[top_card]);

        let is_land = ctx.game.card(top_card).is_land();

        if is_land {
            // Land → put into hand
            let owner = ctx.game.card(top_card).owner;
            ctx.game.move_card(top_card, ZoneType::Hand, owner);
            emit_zone_trigger(
                ctx.trigger_handler,
                top_card,
                ZoneType::Library,
                ZoneType::Hand,
            );
        } else {
            // Nonland → put +1/+1 counter on explorer
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                &ctx.game.cards,
                ctx.game.card(explorer_id),
                &CounterType::P1P1,
            ) {
                ctx.game
                    .card_mut(explorer_id)
                    .add_counter(&CounterType::P1P1, 1);
            }
            ctx.trigger_handler.run_trigger(
                TriggerType::CounterAdded,
                RunParams {
                    card: Some(explorer_id),
                    counter_type: Some("P1P1".to_string()),
                    counter_amount: Some(1),
                    ..Default::default()
                },
                false,
            );

            // Player may put revealed card into graveyard (otherwise it stays on top).
            // Java's ExploreEffect calls controller.confirmAction() which in the
            // harness DeterministicController uses a random boolean (pickBool).
            // Use confirm_action here to match that RNG-consuming path.
            let card_name = ctx.game.card(top_card).card_name.clone();
            let explorer_name = ctx.game.card(explorer_id).card_name.clone();
            let msg = format!(
                "Put {} into your graveyard?",
                card_name
            );
            let put_in_gy = ctx.agents[controller.index()].confirm_action(
                controller,
                None,
                &msg,
                &[],
                Some(&explorer_name),
                Some("Explore"),
            );

            if put_in_gy {
                let owner = ctx.game.card(top_card).owner;
                ctx.game.move_card(top_card, ZoneType::Graveyard, owner);
                emit_zone_trigger(
                    ctx.trigger_handler,
                    top_card,
                    ZoneType::Library,
                    ZoneType::Graveyard,
                );
            }
        }
    }
}
