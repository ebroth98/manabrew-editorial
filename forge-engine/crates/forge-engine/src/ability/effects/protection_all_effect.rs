use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ ProtectionAll` — grant protection to all matching permanents.
///
/// Mirrors Java's `ProtectAllEffect.java`.
/// - `ValidCards$` — filter for which permanents gain protection.
/// - `Gains$` — the protection keyword to grant.
/// - `Choices$` — if present, player chooses the protection quality.
///
/// # Card script examples
/// ```text
/// A:SP$ ProtectionAll | ValidCards$ Creature.YouCtrl | Gains$ Protection from chosen color
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let valid_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Creature.YouCtrl".to_string());
    let gains = sa.params.get("Gains").cloned().unwrap_or_default();

    // If choosing a color, do it once for all targets
    let prot_keyword = if gains.contains("chosen color") {
        let choices = sa
            .params
            .get("Choices")
            .map(|s| {
                s.split(',')
                    .map(|c| c.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![
                    "White".into(),
                    "Blue".into(),
                    "Black".into(),
                    "Red".into(),
                    "Green".into(),
                ]
            });
        let chosen = ctx.agents[controller.index()].choose_color(controller, &choices);
        match chosen {
            Some(color) => format!("Protection from {}", color.to_lowercase()),
            None => return,
        }
    } else {
        gains
    };

    if prot_keyword.is_empty() {
        return;
    }

    let player_ids = ctx.game.player_order.clone();
    let mut targets: Vec<CardId> = Vec::new();

    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_filter, controller) {
                targets.push(cid);
            }
        }
    }

    for cid in targets {
        if ctx.game.card(cid).zone == ZoneType::Battlefield {
            ctx.game
                .card_mut(cid)
                .pump_keywords
                .push(prot_keyword.clone());
        }
    }
}
