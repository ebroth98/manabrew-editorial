//! StoreSVar effect — store a value in an SVar for later use.
//!
//! Ported from Java's `StoreSVarEffect.java`.
//! Stores a calculated value into a named SVar on the source card.

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `StoreSVarEffect` class extending `SpellAbilityEffect`.
pub struct StoreSVarEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for StoreSVarEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let Some(source_id) = sa.source else { return };

    let svar_name = sa
        .params
        .get("SVar")
        .or_else(|| sa.params.get(keys::SVAR_NAME))
        .map(|s| s.to_string())
        .unwrap_or_default();
    let svar_type = sa
        .params
        .get("Type")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let expression = sa
        .params
        .get("Expression")
        .or_else(|| sa.params.get(keys::SVAR_VALUE))
        .map(|s| s.to_string())
        .unwrap_or_default();

    if svar_name.is_empty() || svar_type.is_empty() || expression.is_empty() {
        return;
    }

    let resolved_number = match svar_type.as_str() {
        "Number" => expression
            .parse::<i32>()
            .unwrap_or_else(|_| crate::svar::evaluate_svar(&expression, sa)),
        "Count" => crate::svar::resolve_count_svar_for_sa(
            &expression,
            ctx.game,
            source_id,
            sa.activating_player,
            sa,
        ),
        "Calculate" => {
            if let Some(svar_expr) = ctx.game.card(source_id).get_s_var(&expression) {
                crate::svar::resolve_count_svar_for_sa(
                    svar_expr,
                    ctx.game,
                    source_id,
                    sa.activating_player,
                    sa,
                )
            } else {
                expression
                    .parse::<i32>()
                    .unwrap_or_else(|_| crate::svar::evaluate_svar(&expression, sa))
            }
        }
        _ => expression
            .parse::<i32>()
            .unwrap_or_else(|_| crate::svar::evaluate_svar(&expression, sa)),
    };
    let resolved_value = format!("Number${}", resolved_number);

    ctx.game
        .card_mut(source_id)
        .svars
        .insert(svar_name.clone(), resolved_value);
    }
}
