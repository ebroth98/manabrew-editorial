use super::{resolve_defined_players, EffectContext};
use crate::parsing::keys;
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
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ChoosePlayerEffect` class extending `SpellAbilityEffect`.
pub struct ChoosePlayerEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for ChoosePlayerEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let controller = sa.activating_player;
    let defined = sa.params.get(keys::DEFINED).unwrap_or("You");
    let choosers = resolve_defined_players(defined, controller, ctx.game);

    let valid_players: Vec<_> = if let Some(choices) = sa.params.get(keys::CHOICES) {
        resolve_defined_players(choices, controller, ctx.game)
            .into_iter()
            .filter(|&pid| ctx.game.player(pid).is_alive())
            .collect()
    } else {
        // Match Java getPlayersInTurnOrder() ordering while excluding players
        // no longer in game.
        ctx.game
            .player_order
            .iter()
            .copied()
            .filter(|&pid| ctx.game.player(pid).is_alive())
            .collect()
    };

    for chooser in choosers {
        if !ctx.game.player(chooser).is_alive() {
            continue;
        }
        let chosen =
            ctx.agents[chooser.index()].choose_target_player(chooser, &valid_players, None);

        if let Some(chosen_pid) = chosen {
            if let Some(source_id) = sa.source {
                ctx.game.card_mut(source_id).set_chosen_player(
                    Some(chosen_pid),
                    Some(chooser),
                    !sa.params.is_true(keys::SECRETLY),
                );
            }
        }
    }
    }
}
