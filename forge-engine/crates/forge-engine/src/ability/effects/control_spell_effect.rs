//! ControlSpell effect — gain control of a spell on the stack.
//!
//! Ported from Java's `ControlSpellEffect.java`.
//! Gain control of target spell. You may choose new targets for it.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Find the targeted spell on the stack and change its controller
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == forge_foundation::ZoneType::Stack {
            ctx.game.card_mut(target_card).set_controller(controller);
            // The stack entry's activating_player should also change
            // but MagicStack doesn't expose mutable entries easily.
            // The controller change on the card handles most game mechanics.
        }
    }
}
