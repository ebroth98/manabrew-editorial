use super::{parse_param, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1);
    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);
    ctx.game.draw_cards(target, num as usize);
}
