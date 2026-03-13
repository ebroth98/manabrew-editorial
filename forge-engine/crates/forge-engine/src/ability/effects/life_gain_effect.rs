use super::{resolve_defined_player, resolve_numeric_svar, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::replacement::handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = resolve_numeric_svar(ctx.game, sa, "LifeAmount", 1);
    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);
    if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_gain_life(
        ctx.game, target,
    ) {
        return;
    }
    // Run GainLife replacement effects (e.g. double life gain).
    let mut event = ReplacementEvent::GainLife {
        player: target,
        amount,
    };
    let result = apply_replacements(ctx.game, &mut event);
    let amount = if let ReplacementEvent::GainLife {
        amount: final_amount,
        ..
    } = event
    {
        final_amount
    } else {
        amount
    };
    if result == ReplacementResult::Skipped || amount <= 0 {
        return;
    }
    ctx.game.player_mut(target).gain_life(amount);

    // Fire LifeGained trigger
    ctx.trigger_handler.run_trigger(
        TriggerType::LifeGained,
        RunParams {
            player: Some(target),
            life_amount: Some(amount),
            ..Default::default()
        },
        false,
    );
}
