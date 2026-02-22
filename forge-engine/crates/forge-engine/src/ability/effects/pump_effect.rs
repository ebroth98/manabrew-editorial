use forge_foundation::ZoneType;

use super::{parse_param, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let att_bonus = parse_param(&sa.ability_text, "NumAtt$ ").unwrap_or(0);
    let def_bonus = parse_param(&sa.ability_text, "NumDef$ ").unwrap_or(0);

    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target_card).power_modifier += att_bonus;
            ctx.game.card_mut(target_card).toughness_modifier += def_bonus;
        }
    }
}
