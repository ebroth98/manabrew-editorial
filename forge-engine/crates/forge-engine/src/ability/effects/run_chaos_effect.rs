//! RunChaos — run the chaos ability of the current plane card.
//! Ported from Java's RunChaosEffect: finds ChaosEnsues triggers on
//! target plane cards and fires them.

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Fire ChaosEnsues trigger for target cards
    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        ctx.game.card(source).remembered_cards.clone()
    } else {
        return;
    };

    for card_id in targets {
        ctx.trigger_handler.run_trigger(
            TriggerType::ChaosEnsues,
            RunParams {
                card: Some(card_id),
                player: Some(sa.activating_player),
                ..Default::default()
            },
            false,
        );
    }
}
