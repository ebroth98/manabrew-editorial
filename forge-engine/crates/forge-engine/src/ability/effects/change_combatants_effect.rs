//! ChangeCombatants effect — modify combat participants.
//!
//! Ported from Java's `ChangeCombatantsEffect.java`.
//! Add or remove creatures from combat.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let targets: Vec<crate::ids::CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card.into_iter().collect()
    } else {
        sa.source.into_iter().collect()
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield { continue; }

        if sa.param_is_true(keys::REMOVE_FROM_COMBAT) {
            ctx.game.card_mut(card_id).attacking_player = None;
        }
        if sa.param_is_true(keys::ADD_ATTACKING) {
            let controller = sa.activating_player;
            let defender = ctx.game.opponent_of(controller);
            ctx.game.card_mut(card_id).attacking_player = Some(defender);
        }
    }
}
