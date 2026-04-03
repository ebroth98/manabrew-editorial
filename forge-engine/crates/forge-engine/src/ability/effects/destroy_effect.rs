use forge_foundation::ZoneType;

use super::{emit_zone_trigger_with_lki_counters, EffectContext};
use crate::card::card_util;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let mut targets = Vec::new();
    if let Some(target_card) = sa.target_chosen.target_card {
        targets.push(target_card);
    }
    targets.extend(card_util::get_radiance(ctx.game, sa).iter().copied());
    targets.sort_unstable_by_key(|cid| cid.0);
    targets.dedup();

    for target_card in targets {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            // Indestructible prevents destruction (CR 702.12)
            if ctx.game.card(target_card).has_indestructible() {
                continue;
            }
            let owner = ctx.game.card(target_card).owner;

            // Capture +1/+1 counter count before move (for Modular death triggers)
            let lki_p1p1 = *ctx
                .game
                .card(target_card)
                .counters
                .get(&crate::card::CounterType::P1P1)
                .unwrap_or(&0);

            // Capture LKI counters for death triggers (e.g. Servant of the Scale)
            {
                let lki_counters = ctx.game.card(target_card).counters.clone();
                let lki_power = ctx.game.card(target_card).power();
                let lki_toughness = ctx.game.card(target_card).toughness();
                ctx.game.card_mut(target_card).lki_counters = Some(lki_counters);
                ctx.game.card_mut(target_card).set_lki_power_toughness(Some(lki_power), Some(lki_toughness));
            }
            // Fire Destroyed trigger before moving to graveyard
            ctx.trigger_handler.run_trigger(
                TriggerType::Destroyed,
                RunParams {
                    card: Some(target_card),
                    causer: sa.source,
                    cause_card: sa.source,
                    cause_player: Some(sa.activating_player),
                    ..Default::default()
                },
                false,
            );

            ctx.move_card(target_card, ZoneType::Graveyard, owner);

            emit_zone_trigger_with_lki_counters(
                ctx.trigger_handler,
                target_card,
                ZoneType::Battlefield,
                ZoneType::Graveyard,
                lki_p1p1,
                ctx.game
                    .card(target_card)
                    .lki_power
                    .unwrap_or_else(|| ctx.game.card(target_card).power()),
                ctx.game
                    .card(target_card)
                    .lki_toughness
                    .unwrap_or_else(|| ctx.game.card(target_card).toughness()),
            );
        }
    }
}
