use forge_foundation::ZoneType;

use super::{
    emit_zone_trigger, matches_change_type, parse_param, parse_zone_type, resolve_defined_player,
    resolve_numeric_svar, EffectContext,
};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ DigUntil` — reveal cards from the top of library until finding N matching cards.
///
/// Mirrors Java's `DigUntilEffect.java`.
/// - `Amount$` — how many matching cards to find (default 1).
/// - `Valid$` — filter for matching cards (e.g. "Land", "Creature").
/// - `FoundDestination$` — where found cards go (default Hand).
/// - `RevealedDestination$` — where non-matching cards go (default Library bottom).
///
/// # Card script examples
/// ```text
/// A:SP$ DigUntil | Valid$ Land | FoundDestination$ Hand | RevealedDestination$ Graveyard
/// A:SP$ DigUntil | Valid$ Creature | Amount$ 2 | FoundDestination$ Battlefield
/// ```
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `DigUntilEffect` class extending `SpellAbilityEffect`.
pub struct DigUntilEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for DigUntilEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let amount = parse_param(&sa.ability_text, "Amount$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "Amount", 1))
        as usize;

    let valid_filter = sa
        .params
        .get("Valid")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Card".to_string());

    let found_dest = sa
        .params
        .get("FoundDestination")
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Hand);
    let revealed_dest = sa
        .params
        .get("RevealedDestination")
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Library);

    let target_player = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    let lib_len = ctx
        .game
        .cards_in_zone(ZoneType::Library, target_player)
        .len();
    if lib_len == 0 {
        return;
    }

    let mut found = Vec::new();
    let mut rest = Vec::new();

    // Walk from top of library down
    let lib_cards: Vec<_> = ctx
        .game
        .cards_in_zone(ZoneType::Library, target_player)
        .to_vec();
    // Library is stored bottom→top, so iterate from end (top) backwards
    for &cid in lib_cards.iter().rev() {
        if found.len() >= amount {
            break;
        }
        if matches_change_type(ctx.game.card(cid), &valid_filter, &[]) {
            found.push(cid);
        } else {
            rest.push(cid);
        }
    }

    // Remove found + rest cards from library
    let removed: Vec<_> = found.iter().chain(rest.iter()).copied().collect();
    {
        let zone = ctx.game.zone_mut(ZoneType::Library, target_player);
        zone.cards.retain(|id| !removed.contains(id));
    }

    // Move found cards to destination
    for &id in &found {
        let owner = ctx.game.card(id).owner;
        let dest_owner = if found_dest == ZoneType::Battlefield {
            sa.activating_player
        } else {
            owner
        };
        ctx.move_card(id, found_dest, dest_owner);
        if found_dest == ZoneType::Battlefield {
            let _ = super::add_to_combat(ctx, sa, id, keys::ATTACKING);
        }
        emit_zone_trigger(ctx.trigger_handler, id, ZoneType::Library, found_dest);
    }

    // Move rest to revealed destination
    for &id in &rest {
        let owner = ctx.game.card(id).owner;
        if revealed_dest == ZoneType::Library {
            // Put on bottom
            ctx.game
                .zone_mut(ZoneType::Library, owner)
                .cards
                .insert(0, id);
            ctx.game.card_mut(id).set_zone(ZoneType::Library);
        } else {
            ctx.move_card(id, revealed_dest, owner);
            emit_zone_trigger(ctx.trigger_handler, id, ZoneType::Library, revealed_dest);
        }
    }
    }
}
