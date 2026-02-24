use forge_foundation::ZoneType;

use super::{parse_param, resolve_numeric_svar, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Try direct integer first, then fall back to SVar resolution (for Count$Kicked etc.)
    let att_bonus = parse_param(&sa.ability_text, "NumAtt$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumAtt", 0));
    let def_bonus = parse_param(&sa.ability_text, "NumDef$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumDef", 0));

    // Parse KW$ parameter for keyword grants (e.g. "KW$ Haste" or "KW$ Flying & Trample")
    let keywords: Vec<String> = sa
        .params
        .get("KW")
        .map(|kw_str| {
            kw_str
                .split('&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Overload: apply pump to ALL valid creatures instead of the chosen target.
    if sa.overloaded {
        let valid_tgts = sa.params.get("ValidTgts").cloned().unwrap_or_default();
        let all_bf: Vec<crate::ids::CardId> = ctx.game.player_order.clone().iter()
            .flat_map(|&pid| ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec())
            .collect();
        for cid in all_bf {
            if ctx.game.card(cid).zone != ZoneType::Battlefield {
                continue;
            }
            if !super::matches_valid_cards(ctx.game.card(cid), &valid_tgts, sa.activating_player) {
                continue;
            }
            ctx.game.card_mut(cid).power_modifier += att_bonus;
            ctx.game.card_mut(cid).toughness_modifier += def_bonus;
            for kw in &keywords {
                ctx.game.card_mut(cid).pump_keywords.push(kw.clone());
            }
        }
        return;
    }

    // Resolve the target card: explicit target, Defined$ Self, or Defined$ ParentTarget.
    let target_card = sa.target_chosen.target_card.or_else(|| {
        match sa.params.get("Defined").map(|s| s.as_str()) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => None,
        }
    });

    if let Some(target_card) = target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target_card).power_modifier += att_bonus;
            ctx.game.card_mut(target_card).toughness_modifier += def_bonus;
            for kw in &keywords {
                ctx.game.card_mut(target_card).pump_keywords.push(kw.clone());
            }
        }
    }
}
