//! ChooseGeneric effect — generic modal choice.
//!
//! Ported from Java's `ChooseGenericEffect.java`.
//! Present a list of choices, player picks one, resolve the corresponding sub-ability.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let choices: Vec<String> = sa.params.get("Choices")
        .map(|s| s.split(',').map(|c| c.trim().to_string()).collect())
        .unwrap_or_default();

    if choices.is_empty() { return; }

    // Player chooses
    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
    let chose_first = ctx.agents[controller.index()].confirm_action(
        controller, Some("ChooseGeneric"),
        &format!("Choose: {}", choices.join(" or ")),
        &choices, None, None,
    );

    // Store choice for sub-ability resolution
    if let Some(sid) = sa.source {
        let idx = if chose_first { 0 } else { 1.min(choices.len() - 1) };
        ctx.game.card_mut(sid).svars.insert(
            "ChosenGeneric".to_string(), choices[idx].clone(),
        );
    }
}
