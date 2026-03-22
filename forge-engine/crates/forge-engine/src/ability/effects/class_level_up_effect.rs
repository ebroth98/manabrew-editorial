//! ClassLevelUp effect — increase a Class enchantment's level.
//!
//! Ported 1:1 from Java's `ClassLevelUpEffect.java`.
//! Level up: Increment the Class enchantment's level, activating the
//! next tier of static abilities and triggers.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(host_id) = sa.source else { return };

    let current_level = ctx.game.card(host_id).class_level;
    let new_level = current_level + 1;

    ctx.game.card_mut(host_id).set_class_level(new_level);

    // Re-register triggers for the card at its new level
    ctx.trigger_handler
        .register_active_trigger(ctx.game, host_id);
}
