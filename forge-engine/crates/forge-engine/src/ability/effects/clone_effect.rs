use forge_foundation::ZoneType;

use super::{matches_valid_cards_for_sa, EffectContext};
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
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `CloneEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(CloneEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
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
    let clone_target_id = if let Some(defined) = sa.ir.clone_target.as_deref() {
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
    let duration = crate::parsing::raw_get(&sa.ability_text, crate::parsing::keys::DURATION);
    if duration.is_some() && ctx.game.card(clone_target_id).clone_state.is_none() {
        let mut state = ctx.game.card(clone_target_id).capture_clone_state();
        if let Some(animate_state) = ctx.game.card(clone_target_id).animate_state.as_ref() {
            state.original_type_line = animate_state.original_type_line.clone();
            state.original_base_power = animate_state.original_base_power;
            state.original_base_toughness = animate_state.original_base_toughness;
            state.original_color = animate_state.original_color;
        }
        ctx.game
            .card_mut(clone_target_id)
            .set_clone_state(Some(state));
    }
    let target = &mut ctx.game.cards[clone_target_id.index()];
    crate::card::card_copy_service::copy_copiable_characteristics(&src, target);
    target.add_clone_state();
    target.activated_abilities = src.activated_abilities.clone();
    target.static_abilities = src.static_abilities.clone();
    target.replacement_effects = src.replacement_effects.clone();
    target.set_perpetual(&src, false);

    // Step 4: Apply PumpKeywords$ (extra keywords on the copy)
    if let Some(pump_kws) = sa.ir.pump_keywords.as_deref() {
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

/// End-of-turn revert for clone effects. Mirrors the `GameCommand.run()`
/// anonymous class in Java `CloneEffect` that calls `removeCloneState`,
/// clears imprinted/remembered cards, and restores the original state.
///
/// Removes the clone stamp from the card, reverting copied characteristics
/// and restoring original remembered/imprinted state.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId) {
    if game.card(card_id).zone != ZoneType::Battlefield {
        return;
    }
    // Revert copiable characteristics by clearing clone-specific state.
    // The card's base characteristics (from the card definition) take over.
    let card = game.card_mut(card_id);
    card.remove_clone_state();
    card.imprinted_cards.clear();
    card.remembered_cards.clear();
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
    if let Some(defined) = sa.defined() {
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
    if let Some(filter) = sa.ir.choices.as_deref().map(str::to_string) {
        let filter_selector = sa.ir.choices_selector.as_ref();
        let zone = sa.ir.choice_zone.unwrap_or(ZoneType::Battlefield);

        let mut valid = Vec::new();
        for &pid in &ctx.game.player_order.clone() {
            let zone_cards = ctx.game.cards_in_zone(zone, pid).to_vec();
            for cid in zone_cards {
                if matches_valid_cards_for_sa(
                    ctx.game,
                    sa,
                    ctx.game.card(cid),
                    filter_selector,
                    &filter,
                ) {
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
