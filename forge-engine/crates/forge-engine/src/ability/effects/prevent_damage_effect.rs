use forge_foundation::ZoneType;

use super::{resolve_numeric_svar, EffectContext};
use crate::spellability::SpellAbility;

/// `SP$ PreventDamage` — prevent the next N damage that would be dealt to
/// a target creature or player this turn.
///
/// Mirrors Java's `PreventDamageEffect.java`.
/// - `Amount$` — number of damage to prevent (default 1).
/// - `Defined$` — who gets the shield (Self, Targeted, ParentTarget, You).
///
/// # Card script examples
/// ```text
/// A:SP$ PreventDamage | Defined$ Self | Amount$ 3
/// A:SP$ PreventDamage | Defined$ Targeted | Amount$ X
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = resolve_numeric_svar(ctx.game, sa, "Amount", 1);
    if amount <= 0 { return; }

    // Try targeted creature first
    if let Some(target) = sa.target_chosen.target_card {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).damage_prevention += amount;
        }
        return;
    }

    // Try targeted player
    if let Some(pid) = sa.target_chosen.target_player {
        ctx.game.player_mut(pid).damage_prevention += amount;
        return;
    }

    // Defined$ resolution
    match sa.params.get("Defined").map(|s| s.as_str()) {
        Some("Self") => {
            if let Some(source) = sa.source {
                if ctx.game.card(source).zone == ZoneType::Battlefield {
                    ctx.game.card_mut(source).damage_prevention += amount;
                }
            }
        }
        Some("ParentTarget") => {
            if let Some(parent_target) = ctx.parent_target_card {
                if ctx.game.card(parent_target).zone == ZoneType::Battlefield {
                    ctx.game.card_mut(parent_target).damage_prevention += amount;
                }
            }
        }
        Some("You") => {
            let controller = sa.activating_player;
            ctx.game.player_mut(controller).damage_prevention += amount;
        }
        Some("Opponent") => {
            let opp = ctx.game.opponent_of(sa.activating_player);
            ctx.game.player_mut(opp).damage_prevention += amount;
        }
        _ => {
            // Default: prevent damage to self (source card)
            if let Some(source) = sa.source {
                if ctx.game.card(source).zone == ZoneType::Battlefield {
                    ctx.game.card_mut(source).damage_prevention += amount;
                }
            }
        }
    }
}
