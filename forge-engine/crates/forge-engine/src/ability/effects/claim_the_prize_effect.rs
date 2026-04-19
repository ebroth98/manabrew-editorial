//! ClaimThePrize — Unfinity prize mechanic.
//! Ported from Java's ClaimThePrizeEffect: fires ClaimPrize trigger for
//! each defined attraction.

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source = match sa.source {
        Some(s) => s,
        None => return,
    };

    // Get defined cards (attractions) — defaults to Self
    let attractions = if let Some(def) = sa.params.get(keys::DEFINED) {
        if def == "Self" {
            vec![source]
        } else {
            ctx.game.card(source).remembered_cards.clone()
        }
    } else {
        vec![source]
    };

    for card_id in attractions {
        ctx.trigger_handler.run_trigger(
            TriggerType::ClaimPrize,
            RunParams {
                card: Some(card_id),
                player: Some(sa.activating_player),
                ..Default::default()
            },
            false,
        );
    }
}
