use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// `SP$ RemoveFromCombat` — remove target creature from combat.
///
/// Mirrors Java's `RemoveFromCombatEffect.java`.
/// Simply untaps the creature and removes all combat assignments.
/// The game loop's combat state tracks attackers/blockers externally,
/// so this effect sets the card's tapped state to false and the game loop
/// handles the rest through CombatState filtering.
///
/// # Card script examples
/// ```text
/// A:SP$ RemoveFromCombat | ValidTgts$ Creature
/// ```
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `RemoveFromCombatEffect` class extending `SpellAbilityEffect`.
pub struct RemoveFromCombatEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for RemoveFromCombatEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let target = sa.target_chosen.target_card.or_else(|| {
        match sa.params.get(crate::parsing::keys::DEFINED) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => None,
        }
    });

    if let Some(card_id) = target {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            // Untap the creature (removed from combat means it won't deal/receive combat damage)
            ctx.game.card_mut(card_id).set_tapped(false);
        }
    }
    }
}
