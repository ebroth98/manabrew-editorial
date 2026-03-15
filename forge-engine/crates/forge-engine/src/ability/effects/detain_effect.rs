use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ Detain` — detain target creature(s). Detained creatures can't attack,
/// block, or activate abilities until the controller's next turn.
///
/// Mirrors Java's `DetainEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ Detain | ValidTgts$ Creature
/// A:SP$ Detain | Defined$ Targeted
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Targeted mode
    if let Some(target) = sa.target_chosen.target_card {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).detained = true;
        }
        return;
    }

    // Defined$ mode (e.g. DetainAll pattern)
    if let Some(valid_filter) = sa.params.get("ValidCards") {
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
                ctx.game.card_mut(cid).detained = true;
            }
        }
        return;
    }

    // Defined$ Self
    if let Some(source) = sa.source {
        if ctx.game.card(source).zone == ZoneType::Battlefield {
            ctx.game.card_mut(source).detained = true;
        }
    }
}
