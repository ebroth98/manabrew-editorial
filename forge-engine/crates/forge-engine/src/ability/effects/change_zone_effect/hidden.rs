//! Hidden-origin zone change resolution.
//!
//! Handles library/hand searches with peek, choose, shuffle flow.
//! Mirrors Java's `changeHiddenOriginResolve`.

use forge_foundation::ZoneType;

use super::helpers::{
    can_search_library, find_opposition_agent, find_search_limit,
    matches_with_context, resolve_destination,
};
use super::move_cards::move_cards;
use super::search::{
    resolve_defined_player_choice, resolve_each_search, resolve_multi_search,
    resolve_random_selection, resolve_single_search,
};
use super::super::{resolve_defined_player_with_sa, EffectContext};
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;

/// Resolve zone changes from hidden zones (Library, Hand).
pub(super) fn resolve_hidden_origin(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    origin_zone: ZoneType,
    dest_zone: ZoneType,
) {
    let (dest_zone, lib_position) = resolve_destination(ctx, sa, dest_zone);
    let defined = sa.defined().unwrap_or("").to_string();
    let change_type = sa.change_type().unwrap_or("").to_string();
    let controller = sa.activating_player;
    let change_num = sa.change_num();
    let is_optional = sa.is_optional() || !sa.is_mandatory();

    // ── Defined$ handling (mirrors Java lines 999-1011) ──────────────────
    // When Defined$ is set to a known card reference (Remembered, Imprinted,
    // Self, etc.), we bypass the search flow and directly fetch those cards.
    // Java's changeHiddenOriginResolve checks `defined = sa.hasParam("Defined")`
    // and uses AbilityUtils.getDefinedCards() to get the specific cards.
    let is_defined = !defined.is_empty()
        && !defined.eq_ignore_ascii_case("You")
        && !defined.eq_ignore_ascii_case("Opponent");

    if is_defined && sa.defined_player().is_none() {
        // Resolve defined cards (Remembered, Imprinted, Self, etc.)
        let cards: Vec<crate::ids::CardId> = if defined.eq_ignore_ascii_case("Remembered") {
            sa.source
                .map(|sid| ctx.game.card(sid).remembered_cards.clone())
                .unwrap_or_default()
        } else if defined.eq_ignore_ascii_case("Imprinted") {
            sa.source
                .map(|sid| ctx.game.card(sid).imprinted_cards.clone())
                .unwrap_or_default()
        } else if defined.eq_ignore_ascii_case("Self") {
            sa.source.into_iter().collect()
        } else if defined.eq_ignore_ascii_case("ParentTarget") {
            sa.target_chosen.target_card.into_iter().collect()
        } else if defined.starts_with("TopOfLibrary") {
            // TopOfLibrary, TopOfLibrary2, etc.
            let n = defined
                .strip_prefix("TopOfLibrary")
                .and_then(|s| if s.is_empty() { Some(1) } else { s.parse::<usize>().ok() })
                .unwrap_or(1);
            let lib = ctx.game.cards_in_zone(origin_zone, controller);
            lib.iter().take(n).copied().collect()
        } else if defined.starts_with("BottomOfLibrary") {
            let n = defined
                .strip_prefix("BottomOfLibrary")
                .and_then(|s| if s.is_empty() { Some(1) } else { s.parse::<usize>().ok() })
                .unwrap_or(1);
            let lib = ctx.game.cards_in_zone(origin_zone, controller);
            let len = lib.len();
            lib.iter().skip(len.saturating_sub(n)).copied().collect()
        } else {
            // Unknown defined type — fall through to search
            Vec::new()
        };

        // Filter to only cards still in the expected origin zone
        let valid: Vec<crate::ids::CardId> = cards
            .into_iter()
            .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
            .collect();
        if !valid.is_empty() {
            // For Defined card moves, suppress the post-move library shuffle.
            // Java's changeHiddenOriginResolve checks `!defined` before shuffling
            // (line 1509), so Defined moves never trigger a search shuffle.
            // The shuffle is handled separately by a SubAbility$ DBShuffle if needed.
            let mut sa_no_shuffle = sa.clone();
            sa_no_shuffle.params.insert("NoShuffle".to_string(), "True".to_string());
            move_cards(ctx, &sa_no_shuffle, &valid, origin_zone, dest_zone, &lib_position, controller);
        }
        // For known defined types (Remembered, Imprinted, etc.), always return
        // even if empty — do NOT fall through to a full zone search.
        // In Java, getDefinedCards("Remembered") returns an empty list and
        // the search simply does nothing.
        return;
    }

    // DefinedPlayer$ for hidden-origin
    if sa.defined_player().is_some()
        && !defined.eq_ignore_ascii_case("You")
        && !defined.eq_ignore_ascii_case("Opponent")
    {
        let cards = resolve_defined_player_choice(ctx, sa, origin_zone, &change_type);
        move_cards(ctx, sa, &cards, origin_zone, dest_zone, &lib_position, controller);
        return;
    }

    let search_player = if defined.eq_ignore_ascii_case("Opponent") {
        ctx.game.opponent_of(controller)
    } else {
        controller
    };

    let chooser = if let Some(chooser_def) = sa.chooser() {
        resolve_defined_player_with_sa(chooser_def, sa, controller, ctx.game)
            .unwrap_or(controller)
    } else {
        controller
    };

    // Leonin Arbiter check
    if origin_zone == ZoneType::Library && !can_search_library(ctx, controller) {
        let lib = ctx.game.zone_mut(ZoneType::Library, search_player);
        ctx.rng.shuffle_cards(&mut lib.cards);
        return;
    }

    // Opposition Agent — opponent controls the search
    let effective_chooser = if origin_zone == ZoneType::Library {
        find_opposition_agent(ctx, controller).unwrap_or(chooser)
    } else {
        chooser
    };

    let mut zone_cards = ctx.game.cards_in_zone(origin_zone, search_player).to_vec();

    // Aven Mindcensor restriction
    if origin_zone == ZoneType::Library {
        if let Some(max) = find_search_limit(ctx, search_player, controller) {
            zone_cards.truncate(max);
        }
    }

    // RememberSearched$
    if sa.param_is_true("RememberSearched") {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).remembered_players.push(search_player);
        }
    }

    // Panglacial Wurm — offer to cast while searching
    if origin_zone == ZoneType::Library {
        offer_panglacial_cast(ctx, sa, controller, &mut zone_cards);
    }

    let cards_to_move = if let Some(each_spec) = change_type.strip_prefix("EACH ") {
        resolve_each_search(ctx, sa, each_spec, &mut zone_cards, effective_chooser, is_optional)
    } else {
        let candidates: Vec<_> = zone_cards.iter().copied()
            .filter(|&cid| matches_with_context(ctx, sa, cid, &change_type))
            .collect();
        if candidates.is_empty() {
            Vec::new()
        } else if sa.is_at_random() {
            resolve_random_selection(ctx, &candidates, change_num)
        } else if change_num == 1 {
            resolve_single_search(ctx, sa, &candidates, effective_chooser, is_optional)
        } else {
            resolve_multi_search(ctx, sa, &candidates, effective_chooser, change_num, is_optional)
        }
    };

    // Exactly$ — must find exactly ChangeNum or fail
    if sa.param_is_true("Exactly") && cards_to_move.len() != change_num {
        if origin_zone == ZoneType::Library {
            let lib = ctx.game.zone_mut(ZoneType::Library, search_player);
            ctx.rng.shuffle_cards(&mut lib.cards);
        }
        return;
    }

    // Reveal chosen cards (NoLooking$ suppresses)
    if !sa.param_is_true("NoLooking") && sa.is_reveal() && !cards_to_move.is_empty() && origin_zone == ZoneType::Library {
        for agent in ctx.agents.iter_mut() {
            agent.on_library_peek(ctx.game, &cards_to_move);
        }
    }

    // RememberLKI$
    if sa.param_is_true("RememberLKI") {
        if let Some(sid) = sa.source {
            for &cid in &cards_to_move { ctx.game.card_mut(sid).add_remembered_card(cid); }
        }
    }

    move_cards(ctx, sa, &cards_to_move, origin_zone, dest_zone, &lib_position, controller);
}

/// Offer Panglacial Wurm cast during library search (CR 702.113).
fn offer_panglacial_cast(
    ctx: &mut EffectContext, sa: &SpellAbility, controller: PlayerId, zone_cards: &mut Vec<crate::ids::CardId>,
) {
    let panglacial: Vec<_> = zone_cards.iter().copied()
        .filter(|&cid| ctx.game.card(cid).keywords.iter().any(|k| k.eq_ignore_ascii_case("Panglacial")))
        .collect();
    for pg_id in panglacial {
        let name = ctx.game.card(pg_id).card_name.clone();
        let cast = ctx.agents[controller.index()].confirm_action(
            controller, Some("PanglacialCast"),
            &format!("Cast {} from library while searching?", name),
            &[], Some(&name), None,
        );
        if cast {
            zone_cards.retain(|&cid| cid != pg_id);
            ctx.game.move_card(pg_id, ZoneType::Stack, controller);
        }
    }
}
