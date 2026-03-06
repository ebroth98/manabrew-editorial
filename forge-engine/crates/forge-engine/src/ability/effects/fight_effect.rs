use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// SP$/DB$ Fight — two creatures deal damage to each other equal to their power.
///
/// When `Defined$ ParentTarget` is set (the common pattern for DB$ Fight used by
/// cards like Prey Upon), the first fighter is the parent SA's chosen target card
/// (`ctx.parent_target_card`) and this SA's `target_chosen.target_card` is the
/// second fighter. For a direct SP$ Fight without Defined$, `sa.source` is used
/// as the first fighter.
///
/// Mirrors Java's `FightEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // The explicitly targeted card is always the "other" fighter (opponent's creature).
    let target = match sa.target_chosen.target_card {
        Some(c) => c,
        None => return,
    };

    // Determine the "source" fighter based on Defined$ ParentTarget vs direct source.
    // Prey Upon: `DB$ Fight | Defined$ ParentTarget` — controlled creature from parent SA.
    let is_defined_parent = sa
        .params
        .get("Defined")
        .map(|s| s == "ParentTarget")
        .unwrap_or(false);
    let source = if is_defined_parent {
        match ctx.parent_target_card {
            Some(c) => c,
            None => return,
        }
    } else {
        // Direct SP$ Fight: the source card itself should be a creature (rare pattern).
        match sa.source {
            Some(s) => s,
            None => return,
        }
    };

    // Both must be creatures on the battlefield
    if ctx.game.card(source).zone != ZoneType::Battlefield
        || !ctx.game.card(source).is_creature()
        || ctx.game.card(target).zone != ZoneType::Battlefield
        || !ctx.game.card(target).is_creature()
    {
        return;
    }

    let source_power = ctx.game.card(source).power();
    let target_power = ctx.game.card(target).power();

    // Track damage sources for DamagedBy trigger filters
    if !ctx
        .game
        .card(target)
        .damage_sources_this_turn
        .contains(&source)
    {
        ctx.game
            .card_mut(target)
            .damage_sources_this_turn
            .push(source);
    }
    if !ctx
        .game
        .card(source)
        .damage_sources_this_turn
        .contains(&target)
    {
        ctx.game
            .card_mut(source)
            .damage_sources_this_turn
            .push(target);
    }
    // Deal damage simultaneously
    ctx.game.deal_damage_to_card(target, source_power);
    ctx.game.deal_damage_to_card(source, target_power);

    // Fire Fight triggers
    ctx.trigger_handler.run_trigger(
        TriggerType::Fight,
        RunParams {
            card: Some(source),
            card2: Some(target),
            ..Default::default()
        },
        false,
    );
}
