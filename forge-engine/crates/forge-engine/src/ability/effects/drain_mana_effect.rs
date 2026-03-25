use super::{resolve_defined_players, EffectContext};
use crate::parsing::keys;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

/// Mirrors Java's `DrainManaEffect` for "lose all unspent mana" effects.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let defined = sa
        .params
        .get(crate::parsing::keys::DEFINED)
        .unwrap_or("You");

    let mut drained_total = 0i32;
    // Collect drained mana colors for DrainMana transfer
    let mut drained_mana: Vec<u16> = Vec::new();

    let targets = resolve_defined_players(defined, controller, ctx.game);
    for pid in &targets {
        if !ctx.game.player(*pid).is_alive() {
            continue;
        }
        // Run LoseMana replacement effects before draining mana.
        let mut event = ReplacementEvent::LoseMana { player: *pid };
        let result = apply_replacements(ctx.game, &mut event);
        if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
            continue;
        }
        let pool = &mut ctx.mana_pools[pid.index()];
        let amount = pool.total_mana();
        if amount <= 0 {
            continue;
        }
        // Collect colors before draining (for color-preserving transfer)
        drained_mana.extend(pool.mana_colors());
        drained_total += amount;

        // Mana burn: if player has ManaBurn static, lose life equal to drained mana
        if crate::staticability::static_ability_unspent_mana::has_mana_burn(ctx.game, *pid) {
            ctx.game.player_mut(*pid).life -= amount as i32;
        }

        pool.reset_pool();
    }

    if sa.params.is_true(keys::DRAIN_MANA) && drained_total > 0 {
        // Preserve original colors (mirrors Java behavior)
        for &color in &drained_mana {
            ctx.mana_pools[controller.index()].add_mana(crate::mana::Mana::simple(color));
        }
    }

    if sa.params.is_true(keys::REMEMBER_DRAINED_MANA) {
        if let Some(source_id) = sa.source {
            ctx.game
                .card_mut(source_id)
                .remembered_cmc
                .push(drained_total);
        }
    }
}
