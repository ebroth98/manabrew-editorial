//! Earthbend — turn a land into a 0/0 creature with haste and +1/+1 counters.
//! Ported from Java's EarthbendEffect: adds creature type, haste, and counters
//! to target land, plus sets up a delayed trigger to return it when it dies.

use forge_foundation::{CoreType, ZoneType};

use super::EffectContext;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// Configure the spell ability during construction.
/// Mirrors Java `EarthbendEffect.buildSpellAbility` — sets up targeting to
/// require "Land.YouCtrl" (a land you control).
pub fn build_spell_ability(sa: &mut crate::spellability::SpellAbility) {
    use crate::parsing::Params;
    use crate::spellability::TargetRestrictions;

    // Build target restrictions for "Land.YouCtrl"
    let params = Params::from_raw("ValidTgts$ Land.YouCtrl | TgtPrompt$ land you control");
    if let Some(tr) = TargetRestrictions::new(&params) {
        sa.target_restrictions = Some(tr);
    }
}

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `EarthbendEffect` class extending `SpellAbilityEffect`.
pub struct EarthbendEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for EarthbendEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let num = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);

    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else {
        return;
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }

        // Set base P/T to 0/0
        ctx.game.card_mut(card_id).set_base_pt(Some(0), Some(0));

        // Add Creature core type
        ctx.game
            .card_mut(card_id)
            .add_type(CoreType::Creature.name());

        // Add Haste keyword
        if !ctx
            .game
            .card(card_id)
            .keywords
            .contains_string_ignore_case("Haste")
        {
            ctx.game.card_mut(card_id).add_intrinsic_keyword("Haste");
        }

        // Add +1/+1 counters
        let counter_type = super::parse_counter_type("P1P1");
        ctx.game.card_mut(card_id).add_counter(&counter_type, num);

        // Mark for return-on-death (delayed trigger tracked via svar)
        ctx.game
            .card_mut(card_id)
            .set_s_var("EarthbendReturn", "True");
    }
    }
}
