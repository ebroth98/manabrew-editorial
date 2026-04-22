use super::{resolve_defined_players, EffectContext};
use crate::event::RunParams;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::TriggerType;

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
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ShuffleEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(ShuffleEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let defined = sa.params.get(keys::DEFINED).unwrap_or("You");
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
        ctx.game
            .shuffle_zone_cards(forge_foundation::ZoneType::Library, pid, ctx.rng);
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
