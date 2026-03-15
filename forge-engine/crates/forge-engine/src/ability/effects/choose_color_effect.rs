use super::{resolve_defined_players, EffectContext};
use crate::spellability::SpellAbility;

/// `SP$ ChooseColor` — player(s) choose a color.
///
/// Mirrors Java's `ChooseColorEffect.java`.
///
/// # Params
/// - `Defined$` — which player(s) choose (default: controller/"You")
/// - `Choices` — comma-separated valid colors (default: all 5)
///
/// Stores the chosen color(s) on the source card's `chosen_colors`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let controller = sa.activating_player;

    // Determine which player(s) choose
    let defined = sa
        .params
        .get("Defined")
        .cloned()
        .unwrap_or_else(|| "You".to_string());
    let players = resolve_defined_players(&defined, controller, ctx.game);

    // Valid colors
    let valid_colors: Vec<String> = if let Some(choices) = sa.params.get("Choices") {
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

    if valid_colors.is_empty() {
        return;
    }

    // Clear previous choices on source card
    ctx.game.card_mut(source_id).chosen_colors.clear();

    for player in players {
        ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
        if let Some(chosen) = ctx.agents[player.index()].choose_color(player, &valid_colors) {
            ctx.game.card_mut(source_id).chosen_colors.push(chosen);
        }
    }
}
