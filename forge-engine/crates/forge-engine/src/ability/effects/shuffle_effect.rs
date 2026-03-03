use super::{resolve_defined_players, EffectContext};
use crate::event::{RunParams, TriggerType};
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
        .get("Defined")
        .map(|s| s.as_str())
        .unwrap_or("You");
    let players = resolve_defined_players(defined, sa.activating_player, ctx.game);

    for pid in players {
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
