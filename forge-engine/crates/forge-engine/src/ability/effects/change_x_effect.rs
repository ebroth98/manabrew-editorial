//! ChangeX effect — modify the X value of a spell or ability.
//!
//! Ported from Java's `ChangeXEffect.java`.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else { return };

    let new_x = super::resolve_numeric_svar(ctx.game, sa, "NewX", 0);
    ctx.game
        .card_mut(source_id)
        .svars
        .insert("X".to_string(), format!("Number${}", new_x));
}
