use super::{resolve_defined_players, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ Shuffle` — shuffle one or more players' libraries.
///
/// Mirrors Java's `ShuffleEffect.java`.
/// - `Defined$` selects which player(s) to shuffle (default: You).
///
/// # Card script examples
/// ```text
/// A:SP$ Shuffle | Defined$ You
/// A:SP$ Shuffle | Defined$ Each
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let defined = sa
        .params
        .get(keys::DEFINED)
        .unwrap_or("You");
    let players = resolve_defined_players(defined, sa.activating_player, ctx.game);
    let optional = sa.params.has(keys::OPTIONAL);
    let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());

    for pid in players {
        if optional {
            let accepted = ctx.agents[sa.activating_player.index()].confirm_action(
                sa.activating_player,
                None,
                &format!("Have player {} shuffle their library?", pid.0),
                &[],
                source_name.as_deref(),
                Some(crate::ability::api_type::ApiType::Shuffle),
            );
            if !accepted {
                continue;
            }
        }
        let lib = ctx.game.zone_mut(forge_foundation::ZoneType::Library, pid);
        ctx.rng.shuffle_cards(&mut lib.cards);
        // Fire Shuffled trigger (mirrors Java Player.shuffle)
        ctx.trigger_handler.run_trigger(
            TriggerType::Shuffled,
            RunParams {
                player: Some(pid),
                ..Default::default()
            },
            false,
        );
    }
}
