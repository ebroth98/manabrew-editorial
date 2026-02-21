use forge_foundation::ZoneType;

use super::{parse_num_dmg, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
) {
    let damage = parse_num_dmg(&sa.ability_text);

    // For triggered abilities, resolve Defined$ for target
    let target_player = sa.target_chosen.target_player.or_else(|| {
        if let Some(defined) = sa.params.get("Defined") {
            resolve_defined_player(defined, sa.activating_player, ctx.game)
        } else {
            None
        }
    });

    if let Some(target_player) = target_player {
        ctx.game.deal_damage_to_player(target_player, damage);
    }
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.deal_damage_to_card(target_card, damage);
        }
    }
}
