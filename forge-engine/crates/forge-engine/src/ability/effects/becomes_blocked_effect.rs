//! BecomesBlocked — mark attacking creatures as blocked.
//! Ported from Java's BecomesBlockedEffect: marks attacker as blocked in combat
//! and fires AttackerBlocked triggers.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `BecomesBlockedEffect` class extending `SpellAbilityEffect`.
pub struct BecomesBlockedEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for BecomesBlockedEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Get target cards (attackers to mark as blocked)
    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        // Defined cards from source's remembered
        ctx.game.card(source).remembered_cards.clone()
    } else {
        return;
    };

    for card_id in &targets {
        if ctx.game.card(*card_id).zone != ZoneType::Battlefield {
            continue;
        }
        // Fire AttackerBlocked trigger for each creature that becomes blocked
        ctx.trigger_handler.run_trigger(
            TriggerType::AttackerBlocked,
            RunParams {
                card: Some(*card_id),
                player: Some(sa.activating_player),
                ..Default::default()
            },
            false,
        );
    }
    }
}
