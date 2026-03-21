//! Known-origin zone change resolution.
//!
//! Handles targeted/defined cards from visible zones (Battlefield, Graveyard, etc.)
//! Mirrors Java's `changeKnownOriginResolve`.

use forge_foundation::ZoneType;

use super::helpers::{matches_with_context, resolve_destination};
use super::move_cards::move_cards;
use super::search::resolve_defined_player_choice;
use super::stack::resolve_stack_removal;
use super::super::{resolve_defined_player_with_sa, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// Resolve zone changes from known/visible zones or targeted cards.
pub(super) fn resolve_known_origin(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    origin_zone: ZoneType,
    dest_zone: ZoneType,
) {
    let (dest_zone, lib_position) = resolve_destination(ctx, sa, dest_zone);
    let defined = sa.defined().unwrap_or("").to_string();
    let change_type = sa.change_type().unwrap_or("").to_string();
    let controller = sa.activating_player;

    // Unimprint$ — clear before processing (Java line 506)
    if sa.param_is_true("Unimprint") {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).imprinted_cards.clear();
        }
    }

    // Stack removal path (Java lines 488-500)
    if origin_zone == ZoneType::Stack {
        resolve_stack_removal(ctx, sa, dest_zone, &lib_position, controller);
        return;
    }

    let cards_to_move: Vec<CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card
            .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
            .into_iter().collect()
    } else if defined.eq_ignore_ascii_case("TriggeredNewCardLKICopy")
        || defined.eq_ignore_ascii_case("TriggeredCard")
    {
        sa.trigger_source
            .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
            .into_iter().collect()
    } else if let Some(uid_str) = defined.strip_prefix("CardUID_") {
        uid_str.parse::<u32>().ok().map(crate::ids::CardId)
            .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
            .into_iter().collect()
    } else if defined.eq_ignore_ascii_case("Self")
        || (defined.is_empty() && origin_zone.is_known() && sa.defined_player().is_none())
    {
        sa.source.filter(|&cid| ctx.game.card(cid).zone == origin_zone)
            .into_iter().collect()
    } else if defined.eq_ignore_ascii_case("ExiledWith") {
        resolve_exiled_with(ctx, sa, origin_zone)
    } else if defined.eq_ignore_ascii_case("Remembered") {
        resolve_remembered(ctx, sa, origin_zone)
    } else if sa.defined_player().is_some() {
        resolve_defined_player_choice(ctx, sa, origin_zone, &change_type)
    } else {
        Vec::new()
    };

    move_cards(ctx, sa, &cards_to_move, origin_zone, dest_zone, &lib_position, controller);
}

fn resolve_exiled_with(ctx: &EffectContext, sa: &SpellAbility, origin_zone: ZoneType) -> Vec<CardId> {
    let Some(source_id) = sa.source else { return Vec::new() };
    let mut result: Vec<_> = ctx.game.cards.iter()
        .filter(|c| c.zone == origin_zone && c.exiled_by == Some(source_id))
        .map(|c| c.id).collect();
    let remembered: Vec<_> = ctx.game.card(source_id).remembered_cards.iter().copied()
        .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
        .filter(|cid| !result.contains(cid)).collect();
    result.extend(remembered);
    result
}

fn resolve_remembered(ctx: &EffectContext, sa: &SpellAbility, origin_zone: ZoneType) -> Vec<CardId> {
    let Some(source_id) = sa.source else { return Vec::new() };
    ctx.game.card(source_id).remembered_cards.iter().copied()
        .filter(|&cid| ctx.game.card(cid).zone == origin_zone).collect()
}
