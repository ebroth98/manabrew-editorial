//! DayTime effect — switch between Day and Night.
//!
//! Ported 1:1 from Java's `DayTimeEffect.java`.
//! Day/Night cycle: set the game to Day, Night, or Switch.

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `DayTimeEffect` class extending `SpellAbilityEffect`.
pub struct DayTimeEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for DayTimeEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let value = sa
        .params
        .get(crate::parsing::keys::VALUE)
        .unwrap_or("")
        .to_string();

    match value.as_str() {
        "Day" => {
            ctx.game.day_night_started = true;
            ctx.game.is_night = false;
        }
        "Night" => {
            ctx.game.day_night_started = true;
            ctx.game.is_night = true;
        }
        "Switch" => {
            ctx.game.day_night_started = true;
            ctx.game.is_night = !ctx.game.is_night;
        }
        _ => {}
    }

    // Day/Night changes trigger DFC transformations — handled by the game loop's
    // state-based actions which check is_night against each DFC card.
    }
}
