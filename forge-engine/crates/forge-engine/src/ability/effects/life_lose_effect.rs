use super::{parse_param, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = parse_param(&sa.ability_text, "LifeAmount$ ").unwrap_or(1);
    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);
    if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_lose_life(ctx.game, target) {
        return;
    }
    ctx.game.player_mut(target).lose_life(amount);

    // Fire LifeLost trigger
    ctx.trigger_handler.run_trigger(
        TriggerType::LifeLost,
        RunParams {
            player: Some(target),
            life_amount: Some(amount),
            ..Default::default()
        },
        false,
    );
}
