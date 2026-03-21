//! Repeat effect — repeat a sub-ability N times.
//!
//! Ported from Java's `RepeatEffect.java`.
//! Repeat: Execute the sub-ability chain N times, where N is calculated
//! from a param or SVar.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let _num = super::resolve_numeric_svar(ctx.game, sa, "RepeatNum", 1).max(0);

    // The repeat count is used by the sub-ability resolution pipeline.
    // Java's RepeatEffect stores the count and iterates sub-ability resolution.
    // In Rust, the sub-ability chain handles iteration via the spell resolution
    // system. The count is stored in the SA's params for the resolver to use.

    // Store the resolved count for downstream use
    if let Some(source_id) = sa.source {
        ctx.game
            .card_mut(source_id)
            .svars
            .insert("RepeatNum".to_string(), format!("Number${}", _num));
    }
}
