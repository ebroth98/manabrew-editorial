use forge_foundation::ZoneType;

use super::{parse_param, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// Mirrors the `RearrangeTopOfLibrary` API used by cards like Ponder.
///
/// `SP$ RearrangeTopOfLibrary | Defined$ You | NumCards$ N | MayShuffle$ True`
/// The activating player looks at the top N cards and puts them back in any order.
/// With `MayShuffle$ True`, the player may choose to shuffle instead.
///
/// The agent decides the order via `choose_reorder_library`.
/// The default implementation (PassAgent) keeps the existing order.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(3) as usize;
    let may_shuffle = sa
        .params
        .get("MayShuffle")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);

    let lib_len = ctx.game.cards_in_zone(ZoneType::Library, target).len();
    if lib_len == 0 {
        return;
    }

    let count = num.min(lib_len);

    // Take top N cards (last `count` elements).
    let top_n: Vec<_> = {
        let zone = ctx.game.zone_mut(ZoneType::Library, target);
        let len = zone.cards.len();
        zone.cards.split_off(len - count)
    };

    // Ask the agent to reorder the cards.
    let reordered = ctx.agents[sa.activating_player.index()]
        .choose_reorder_library(sa.activating_player, &top_n);

    // Validate: use reordered if it contains exactly the same cards.
    let put_back =
        if reordered.len() == top_n.len() && top_n.iter().all(|id| reordered.contains(id)) {
            reordered
        } else {
            top_n
        };

    // Put cards back on top (append to end = top of library).
    for &id in &put_back {
        ctx.game.zone_mut(ZoneType::Library, target).cards.push(id);
    }

    // Handle optional shuffle (default: no shuffle).
    if may_shuffle {
        let wants_shuffle =
            ctx.agents[sa.activating_player.index()].choose_may_shuffle(sa.activating_player);
        if wants_shuffle {
            let mut rng = rand::thread_rng();
            ctx.game.shuffle_library(target, &mut rng);
            ctx.trigger_handler.run_trigger(
                TriggerType::Shuffled,
                RunParams {
                    player: Some(target),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
