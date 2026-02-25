use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::spellability::SpellAbility;

/// `SP$ ControlGainVariant` — complex control redistribution.
///
/// Mirrors Java's `ControlGainVariantEffect.java`.
///
/// # Params
/// - `ChangeController` — mode: "CardOwner", "Random", etc.
/// - `AllValid` — filter for which permanents are affected
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let mode = sa
        .params
        .get("ChangeController")
        .cloned()
        .unwrap_or_else(|| "CardOwner".to_string());

    let filter = sa
        .params
        .get("AllValid")
        .cloned()
        .unwrap_or_else(|| "Permanent".to_string());

    // Collect all matching permanents on the battlefield
    let matching: Vec<crate::ids::CardId> = {
        let mut result = Vec::new();
        for &pid in &ctx.game.player_order.clone() {
            let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
            for cid in zone_cards {
                if matches_valid_cards(ctx.game.card(cid), &filter, controller) {
                    result.push(cid);
                }
            }
        }
        result
    };

    match mode.as_str() {
        "CardOwner" => {
            // Homeward Path: return each permanent to its owner's control
            for cid in matching {
                let owner = ctx.game.card(cid).owner;
                if ctx.game.card(cid).can_be_controlled_by(owner) {
                    ctx.game.change_controller(cid, owner);
                }
            }
        }
        "Random" => {
            // Scrambleverse: assign each permanent to a random player
            use rand::Rng;
            let alive = ctx.game.alive_players();
            if alive.is_empty() {
                return;
            }
            let mut rng = rand::thread_rng();
            for cid in matching {
                let random_idx = rng.gen_range(0..alive.len());
                let new_controller = alive[random_idx];
                if ctx.game.card(cid).can_be_controlled_by(new_controller) {
                    ctx.game.change_controller(cid, new_controller);
                }
            }
        }
        _ => {
            // Other modes (multiplayer-specific) are logged and skipped
            eprintln!(
                "[ControlGainVariant] Unimplemented mode: {}",
                mode
            );
        }
    }
}
