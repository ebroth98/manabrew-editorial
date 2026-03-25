//! StoreSVar effect — store a value in an SVar for later use.
//!
//! Ported from Java's `StoreSVarEffect.java`.
//! Stores a calculated value into a named SVar on the source card.

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else { return };

    let svar_name = sa
        .params
        .get(keys::SVAR_NAME)
        .map(|s| s.to_string())
        .unwrap_or_default();
    let svar_value = sa
        .params
        .get(keys::SVAR_VALUE)
        .map(|s| s.to_string())
        .unwrap_or_default();

    if svar_name.is_empty() {
        return;
    }

    // Calculate the value if it's a numeric expression
    let resolved_value = if let Ok(n) = svar_value.parse::<i32>() {
        format!("Number${}", n)
    } else {
        let calculated = super::resolve_numeric_svar(ctx.game, sa, "SVarValue", 0);
        format!("Number${}", calculated)
    };

    ctx.game
        .card_mut(source_id)
        .svars
        .insert(svar_name, resolved_value);
}
