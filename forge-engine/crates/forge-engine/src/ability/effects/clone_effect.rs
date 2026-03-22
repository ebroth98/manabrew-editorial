use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_zone_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ Clone` — one card becomes a copy of another.
///
/// Mirrors Java's `CloneEffect.java`.
///
/// # Params
/// - `Choices` — filter for valid clone sources (if player picks)
/// - `ChoiceZone` — zone to pick from (default Battlefield)
/// - `Defined$` — resolve defined cards as the clone source
/// - `CloneTarget` — defined cards to be cloned onto (default: source card)
/// - `PumpKeywords` — extra keywords on the copy
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let controller = sa.activating_player;

    // Step 1: Determine the clone source (what to copy FROM)
    let clone_source = resolve_clone_source(ctx, sa, controller);
    let clone_source_id = match clone_source {
        Some(id) => id,
        None => return,
    };

    // Step 2: Determine the clone target (what to copy ONTO)
    let clone_target_id = if let Some(defined) = sa.params.get(keys::CLONE_TARGET) {
        match defined {
            "Self" => source_id,
            "ParentTarget" => ctx.parent_target_card.unwrap_or(source_id),
            "Remembered" => ctx
                .game
                .card(source_id)
                .remembered_cards
                .first()
                .copied()
                .unwrap_or(source_id),
            _ => source_id,
        }
    } else {
        // Default: the source card itself (the creature entering as a clone)
        source_id
    };

    // Step 3: Copy characteristics from source → target
    let src = ctx.game.card(clone_source_id).clone();
    let target = &mut ctx.game.cards[clone_target_id.index()];
    crate::card::card_copy_service::copy_copiable_characteristics(&src, target);
    target.activated_abilities = src.activated_abilities.clone();
    target.static_abilities = src.static_abilities.clone();
    target.replacement_effects = src.replacement_effects.clone();
    target.set_perpetual(&src, false);

    // Step 4: Apply PumpKeywords$ (extra keywords on the copy)
    if let Some(pump_kws) = sa.params.get(keys::PUMP_KEYWORDS) {
        for kw in pump_kws.split(',') {
            let kw = kw.trim();
            if !kw.is_empty() {
                ctx.game.card_mut(clone_target_id).add_intrinsic_keyword(kw);
            }
        }
    }

    // Step 5: Re-register triggers for the cloned card
    ctx.trigger_handler
        .register_active_trigger(ctx.game, clone_target_id);
}

/// Determine which card to copy FROM.
fn resolve_clone_source(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    controller: crate::ids::PlayerId,
) -> Option<crate::ids::CardId> {
    // Check explicit target first
    if let Some(target) = sa.target_chosen.target_card {
        return Some(target);
    }

    // Check Defined$
    if let Some(defined) = sa.params.get(keys::DEFINED) {
        match defined {
            "Remembered" => {
                if let Some(src) = sa.source {
                    return ctx.game.card(src).remembered_cards.first().copied();
                }
            }
            "ParentTarget" => {
                return ctx.parent_target_card;
            }
            _ => {}
        }
    }

    // Check Choices — player selects from valid cards
    if let Some(filter) = sa.params.get(keys::CHOICES).map(|s| s.to_string()) {
        let zone = sa
            .params
            .get(keys::CHOICE_ZONE)
            .and_then(|s| parse_zone_type(s))
            .unwrap_or(ZoneType::Battlefield);

        let mut valid = Vec::new();
        for &pid in &ctx.game.player_order.clone() {
            let zone_cards = ctx.game.cards_in_zone(zone, pid).to_vec();
            for cid in zone_cards {
                if matches_valid_cards(ctx.game.card(cid), &filter, controller) {
                    valid.push(cid);
                }
            }
        }

        if valid.is_empty() {
            return None;
        }

        ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
        let chosen =
            ctx.agents[controller.index()].choose_cards_for_effect(controller, &valid, 1, 1);
        return chosen.first().copied();
    }

    None
}
