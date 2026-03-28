//! DamagePrevent effect — prevent the next N damage to a target.
//!
//! Ported from Java's `DamagePreventEffect.java`.
//! Prevent the next N damage that would be dealt to target creature/player.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(0);

    // Target a player
    if let Some(pid) = sa.target_chosen.target_player {
        ctx.game.player_add_damage_prevention(pid, amount);
        return;
    }

    // Target a creature
    if let Some(cid) = sa.target_chosen.target_card {
        if ctx.game.card(cid).zone == ZoneType::Battlefield {
            ctx.game.card_mut(cid).damage_prevention += amount;
        }
        return;
    }

    // Self or defined
    if let Some(def) = sa.defined() {
        if def.eq_ignore_ascii_case("You") || def.eq_ignore_ascii_case("Self") {
            let controller = sa.activating_player;
            ctx.game.player_add_damage_prevention(controller, amount);
        }
    }
}
