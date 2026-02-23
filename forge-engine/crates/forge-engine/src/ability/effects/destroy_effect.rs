use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            let owner = ctx.game.card(target_card).owner;

            // Fire Destroyed trigger before moving to graveyard
            ctx.trigger_handler.run_trigger(
                TriggerType::Destroyed,
                RunParams {
                    card: Some(target_card),
                    cause_player: Some(sa.activating_player),
                    ..Default::default()
                },
                false,
            );

            ctx.game.move_card(target_card, ZoneType::Graveyard, owner);

            emit_zone_trigger(
                ctx.trigger_handler,
                target_card,
                ZoneType::Battlefield,
                ZoneType::Graveyard,
            );
        }
    }
}
