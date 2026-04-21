//! GainOwnership — change ownership of a card (rare silver-bordered).
//! Ported from Java's OwnershipGainEffect: changes the owner of target cards
//! to the defined player.

use super::EffectContext;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `OwnershipGainEffect` class extending `SpellAbilityEffect`.
pub struct OwnershipGainEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for OwnershipGainEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let new_owner = if let Some(def) = sa.params.get(keys::DEFINED_PLAYER) {
        let players = super::resolve_defined_players(def, sa.activating_player, ctx.game);
        players.into_iter().next().unwrap_or(sa.activating_player)
    } else {
        sa.activating_player
    };

    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        ctx.game.card(source).remembered_cards.clone()
    } else {
        return;
    };

    for card_id in targets {
        // Change ownership — in Magic this is extremely rare (silver-bordered only)
        ctx.game.card_mut(card_id).set_owner(new_owner);
        ctx.game.card_mut(card_id).set_controller(new_owner);
    }
    }
}
