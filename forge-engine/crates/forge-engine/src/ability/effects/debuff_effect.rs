//! Debuff — reduce stats permanently (digital-only).

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// End-of-turn revert for debuff. Mirrors the `GameCommand.run()` in Java
/// `DebuffEffect` that restores the original P/T when the effect expires.
///
/// Reverses the debuff by adding back the debuff amount to power/toughness modifiers.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId, amount: i32) {
    if game.card(card_id).zone == forge_foundation::ZoneType::Battlefield {
        game.card_mut(card_id).power_modifier += amount;
        game.card_mut(card_id).toughness_modifier += amount;
    }
}

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `DebuffEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(DebuffEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let amount = sa
        .params
        .get("Num")
        .map(|_| super::resolve_numeric_svar(ctx.game, sa, "Num", 0).max(0))
        .unwrap_or(0);
    let removed_keywords: Vec<String> = sa
        .params
        .get(keys::KEYWORDS)
        .map(|kw_str| {
            kw_str
                .split('&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let permanent = sa
        .params
        .get("Duration")
        .map(|d| d.eq_ignore_ascii_case("Permanent"))
        .unwrap_or(false);
    let target = sa.target_chosen.target_card.or_else(|| match sa.defined() {
        Some("Self") => sa.source,
        Some("ParentTarget") => ctx.parent_target_card,
        Some(_) => None,
        None if !sa.uses_targeting() => sa.source,
        None => None,
    });
    if let Some(target) = target {
        if amount > 0 {
            ctx.game.card_mut(target).power_modifier -= amount;
            ctx.game.card_mut(target).toughness_modifier -= amount;
        }
        for kw in removed_keywords {
            if permanent {
                ctx.game.card_mut(target).remove_intrinsic_keyword(&kw);
            } else {
                ctx.game.card_mut(target).add_cant_have_keyword(&kw);
            }
        }
    }
}
