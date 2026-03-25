use super::{resolve_defined_player_with_sa, resolve_numeric_svar, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = resolve_numeric_svar(ctx.game, sa, "LifeAmount", 1);
    // Mirror Java getTargetPlayers(): targeted player first, then Defined, then activator.
    let target = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player_with_sa(d, sa, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);
    if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_lose_life(
        ctx.game, target,
    ) {
        return;
    }

    // Run LifeReduced replacement effects before losing life.
    let mut event = ReplacementEvent::LifeReduced {
        player: target,
        amount,
        is_damage: false,
    };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
        return;
    }
    let amount = if let ReplacementEvent::LifeReduced {
        amount: final_amount,
        ..
    } = event
    {
        final_amount
    } else {
        amount
    };
    if amount <= 0 {
        return;
    }

    ctx.game.player_mut(target).lose_life(amount);

    // Set AFLifeLost SVar on source card so chained sub-abilities (e.g. GainLife) can read it.
    // Mirrors Java's `sa.setSVar("AFLifeLost", "Number$" + lifeLost)`.
    if let Some(source_id) = sa.source {
        ctx.game
            .card_mut(source_id)
            .svars
            .insert("AFLifeLost".to_string(), format!("Number${}", amount));
    }

    // Fire LifeLost trigger
    ctx.trigger_handler.run_trigger(
        TriggerType::LifeLost,
        RunParams {
            player: Some(target),
            life_amount: Some(amount),
            first_time: Some(ctx.game.player(target).life_lost_this_turn == amount),
            source_card: sa.source,
            source_sa: Some(sa.clone()),
            ..Default::default()
        },
        false,
    );
}
