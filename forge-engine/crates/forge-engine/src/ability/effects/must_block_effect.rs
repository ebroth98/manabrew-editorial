use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// End-of-turn revert for must-block. Mirrors the `GameCommand.run()` in Java
/// `MustBlockEffect` that clears the must-block flag when the effect expires.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId) {
    if game.card(card_id).zone == ZoneType::Battlefield {
        game.card_mut(card_id).set_must_block(false);
    }
}

/// `SP$ MustBlock` — target creature must block this turn if able.
///
/// Mirrors Java's `MustBlockEffect.java` (simplified — sets flag only;
/// full "must block specific attacker" support deferred).
///
/// # Card script examples
/// ```text
/// A:SP$ MustBlock | ValidTgts$ Creature
/// A:SP$ MustBlock | Defined$ Targeted
/// A:SP$ MustBlock | ValidCards$ Creature.OppCtrl
/// ```
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `MustBlockEffect` class extending `SpellAbilityEffect`.
pub struct MustBlockEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for MustBlockEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Targeted mode
    if let Some(target) = sa.target_chosen.target_card {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).set_must_block(true);
        }
        return;
    }

    // ValidCards$ mode (mass must-block)
    if let Some(valid_filter) = sa.params.get(crate::parsing::keys::VALID_CARDS) {
        let player_ids = ctx.game.player_order.clone();
        let mut targets: Vec<CardId> = Vec::new();
        for &pid in &player_ids {
            let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
            for cid in zone_cards {
                if matches_valid_cards(ctx.game.card(cid), valid_filter, sa.activating_player) {
                    targets.push(cid);
                }
            }
        }
        for cid in targets {
            if ctx.game.card(cid).zone == ZoneType::Battlefield {
                ctx.game.card_mut(cid).set_must_block(true);
            }
        }
        return;
    }

    // Defined$ Self
    if let Some(source) = sa.source {
        if ctx.game.card(source).zone == ZoneType::Battlefield {
            ctx.game.card_mut(source).set_must_block(true);
        }
    }
    }
}
