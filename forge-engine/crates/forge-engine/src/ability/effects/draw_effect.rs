use super::{parse_param, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1);
    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);
    let drawn = ctx.game.draw_cards(target, num as usize);

    // Fire Drawn trigger per card
    for card_id in drawn {
        ctx.trigger_handler.run_trigger(
            TriggerType::Drawn,
            RunParams {
                card: Some(card_id),
                player: Some(target),
                ..Default::default()
            },
            false,
        );
    }
}
