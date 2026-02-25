use super::EffectContext;
use crate::spellability::SpellAbility;

/// `SP$ ChoosePlayer` — the activating player chooses a player.
/// Stores the result in `source.chosen_player` for subsequent effects.
///
/// Mirrors Java's `ChoosePlayerEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ ChoosePlayer | Defined$ You
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let valid_players = ctx.game.alive_players();

    let chosen = ctx.agents[controller.index()]
        .choose_target_player(controller, &valid_players);

    if let Some(chosen_pid) = chosen {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).chosen_player = Some(chosen_pid);
        }
    }
}
