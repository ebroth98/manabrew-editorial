use super::{resolve_defined_players, EffectContext};
use crate::spellability::SpellAbility;

/// Mirrors Java's `DrainManaEffect` for "lose all unspent mana" effects.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let defined = sa
        .params
        .get("Defined")
        .map(String::as_str)
        .unwrap_or("You");

    let mut drained_total = 0;
    let targets = resolve_defined_players(defined, controller, ctx.game);
    for pid in targets {
        if !ctx.game.player(pid).is_alive() {
            continue;
        }
        let pool = &mut ctx.mana_pools[pid.index()];
        let amount = pool.total();
        if amount <= 0 {
            continue;
        }
        drained_total += amount;
        pool.empty();
    }

    if sa
        .params
        .get("DrainMana")
        .is_some_and(|v| v.eq_ignore_ascii_case("True"))
        && drained_total > 0
    {
        // Java preserves exact colors; Rust keeps it colorless for now.
        ctx.mana_pools[controller.index()].add(forge_foundation::mana::ManaAtom::COLORLESS, drained_total);
    }

    if sa
        .params
        .get("RememberDrainedMana")
        .is_some_and(|v| v.eq_ignore_ascii_case("True"))
    {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).remembered_cmc.push(drained_total);
        }
    }
}

