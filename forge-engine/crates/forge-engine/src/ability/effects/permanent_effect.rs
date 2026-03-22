//! PermanentEffect — move spell to battlefield.
//!
//! Mirrors Java's `PermanentEffect.java`.
//! Handles moving a spell from the stack to the battlefield as a permanent.
//! This is the parent logic for both PermanentCreatureEffect and
//! PermanentNoncreatureEffect.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Resolve a permanent entering the battlefield.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    resolve_permanent_common(ctx, sa);
}

/// Shared implementation for Permanent, PermanentCreature and PermanentNoncreature.
/// Both extend PermanentEffect in Java, which simply moves the host to play.
pub fn resolve_permanent_common(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source = match sa.source {
        Some(s) => s,
        None => return,
    };

    let controller = sa.activating_player;

    // Check if it should enter tapped (sneak/dash)
    if sa.param_is_true(keys::SNEAK) || sa.param_is_true(keys::TAPPED) {
        ctx.game.card_mut(source).set_tapped(true);
    }

    // Move host card to battlefield
    let old_zone = ctx.game.card(source).zone;
    if old_zone != ZoneType::Battlefield {
        ctx.game.move_card(source, ZoneType::Battlefield, controller);
        super::emit_zone_trigger(ctx.trigger_handler, source, old_zone, ZoneType::Battlefield);
    }
}
