use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ Goad` — goad target creature(s). Goaded creatures must attack each
/// combat if able, and can't attack the player who goaded them.
///
/// Mirrors Java's `GoadEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ Goad | ValidTgts$ Creature.OppCtrl
/// A:SP$ Goad | ValidCards$ Creature.OppCtrl
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Targeted mode
    if let Some(target) = sa.target_chosen.target_card {
        if ctx.game.card(target).zone == ZoneType::Battlefield {
            ctx.game.card_mut(target).goaded_by = Some(controller);
        }
        return;
    }

    // Mass goad: ValidCards$ filter
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
                ctx.game.card_mut(cid).goaded_by = Some(controller);
            }
        }
        return;
    }

    // Defined$ Self fallback
    if let Some(source) = sa.source {
        if ctx.game.card(source).zone == ZoneType::Battlefield {
            ctx.game.card_mut(source).goaded_by = Some(controller);
        }
    }
}
