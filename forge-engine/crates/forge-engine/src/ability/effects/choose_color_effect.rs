use super::{resolve_defined_players, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ ChooseColor` — player(s) choose a color.
///
/// Mirrors Java's `ChooseColorEffect.java`.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ChooseColorEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(ChooseColorEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let controller = sa.activating_player;
    let defined = sa
        .params
        .get(keys::DEFINED)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "You".to_string());
    let players = resolve_defined_players(&defined, controller, ctx.game);

    let mut valid_colors: Vec<String> = if let Some(choices) = sa.params.get(keys::CHOICES) {
        choices
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![
            "White".to_string(),
            "Blue".to_string(),
            "Black".to_string(),
            "Red".to_string(),
            "Green".to_string(),
        ]
    };

    if let Some(exclude) = sa.params.get("Exclude") {
        for excluded in exclude.split(',').map(|s| s.trim()) {
            valid_colors.retain(|c| !c.eq_ignore_ascii_case(excluded));
        }
    }

    if valid_colors.is_empty() {
        return;
    }

    let count_min = if sa.params.has("UpTo") {
        0
    } else if sa.params.has("TwoColors") {
        2
    } else {
        1
    };
    let count_max = if sa.params.has("TwoColors") {
        2
    } else if sa.params.has("OrColors") {
        valid_colors.len()
    } else {
        1
    };

    ctx.game.card_mut(source_id).clear_chosen_colors();

    for player in players {
        ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
        let chosen =
            ctx.agents[player.index()].choose_colors(player, &valid_colors, count_min, count_max);
        if chosen.is_empty() {
            return;
        }
        let card = ctx.game.card_mut(source_id);
        card.clear_chosen_colors();
        for color in chosen {
            card.add_chosen_color(color);
        }
    }
}
