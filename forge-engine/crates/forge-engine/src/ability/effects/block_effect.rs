//! Block effect — force a creature to block a specific attacker.
//!
//! Ported from Java's `BlockEffect.java`.
//! Target creature blocks target attacker this combat if able.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `BlockEffect` class extending `SpellAbilityEffect`.
pub struct BlockEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for BlockEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let targets: Vec<crate::ids::CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card.into_iter().collect()
    } else if let Some(def) = sa.defined() {
        if def.eq_ignore_ascii_case("Self") {
            sa.source.into_iter().collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }
        // Mark creature as "must block" — the combat system checks this flag
        ctx.game.card_mut(card_id).set_must_block(true);
    }
    }
}
