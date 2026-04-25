//! Hidden-origin zone change resolution.
//!
//! Handles library/hand searches with peek, choose, shuffle flow.
//! Mirrors Java's `changeHiddenOriginResolve`.

use forge_foundation::ZoneType;

use super::super::EffectContext;
use super::helpers::{
    can_search_library, find_opposition_agent, find_search_limit, matches_with_context,
    resolve_destination,
};
use super::move_cards::move_cards;
use super::search::{
    resolve_defined_players_for_hidden_origin, resolve_each_search, resolve_multi_search,
    resolve_random_selection, resolve_single_search,
};
use crate::agent::GameEntity;
use crate::ids::PlayerId;
use crate::parsing::keys;
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
    // Java parity: searches are mandatory by default. Only treat as optional
    // when Optional$ True is explicitly set on the ability (e.g. "you may search").
    // Previously `!sa.is_mandatory()` made everything optional by default,
    // causing the agent to decline mandatory searches like Fabled Passage.
    let is_optional = sa.is_optional();

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
                .and_then(|s| {
                    if s.is_empty() {
                        Some(1)
                    } else {
                        s.parse::<usize>().ok()
                    }
                })
                .unwrap_or(1);
            let lib = ctx.game.cards_in_zone(origin_zone, controller);
            lib.iter().rev().take(n).copied().collect()
        } else if defined.starts_with("BottomOfLibrary") {
            let n = defined
                .strip_prefix("BottomOfLibrary")
                .and_then(|s| {
                    if s.is_empty() {
                        Some(1)
                    } else {
                        s.parse::<usize>().ok()
                    }
                })
                .unwrap_or(1);
            let lib = ctx.game.cards_in_zone(origin_zone, controller);
            lib.iter().take(n).copied().collect()
        } else if defined.eq_ignore_ascii_case("DelayTriggerRememberedLKI")
            || defined.eq_ignore_ascii_case("RememberedLKI")
        {
            sa.trigger_objects
                .get(&crate::ability::AbilityKey::RememberedLKI)
                .into_iter()
                .flat_map(|value| value.split(','))
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .map(crate::ids::CardId)
                .collect()
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
            let mut ordered = valid;
            if sa.ir.reorder && ordered.len() > 1 {
                ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
                ctx.agents[controller.index()].on_library_peek(ctx.game, &ordered);
                let reordered =
                    ctx.agents[controller.index()].choose_reorder_library(controller, &ordered);
                if reordered.len() == ordered.len()
                    && ordered.iter().all(|id| reordered.contains(id))
                {
                    ordered = reordered;
                }
            }
            // For Defined card moves, suppress the post-move library shuffle.
            // Java's changeHiddenOriginResolve checks `!defined` before shuffling
            // (line 1509), so Defined moves never trigger a search shuffle.
            // The shuffle is handled separately by a SubAbility$ DBShuffle if needed.
            let mut sa_no_shuffle = sa.clone();
            sa_no_shuffle
                .params
                .put("NoShuffle".to_string(), "True".to_string());
            sa_no_shuffle.ir.no_shuffle = true;
            move_cards(
                ctx,
                &sa_no_shuffle,
                &ordered,
                origin_zone,
                dest_zone,
                &lib_position,
                controller,
            );
        }
        // For known defined types (Remembered, Imprinted, etc.), always return
        // even if empty — do NOT fall through to a full zone search.
        // In Java, getDefinedCards("Remembered") returns an empty list and
        // the search simply does nothing.
        return;
    }

    let chooser = if let Some(chooser_def) = sa.chooser() {
        let chooser_players = crate::ability::ability_utils::resolve_defined_players_with_sa(
            chooser_def,
            sa,
            controller,
            ctx.game,
        );
        if chooser_players.is_empty() {
            controller
        } else {
            let chooser_entities: Vec<_> = chooser_players
                .iter()
                .copied()
                .map(GameEntity::Player)
                .collect();
            ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
            match ctx.agents[controller.index()].choose_single_entity_for_effect(
                controller,
                &chooser_entities,
                false,
            ) {
                Some(GameEntity::Player(pid)) => pid,
                _ => chooser_players[0],
            }
        }
    } else {
        controller
    };

    // DefinedPlayer$ for hidden-origin
    if sa.defined_player().is_some()
        && !defined.eq_ignore_ascii_case("You")
        && !defined.eq_ignore_ascii_case("Opponent")
    {
        for affected_player in resolve_defined_players_for_hidden_origin(ctx, sa) {
            let effective_chooser = if origin_zone == ZoneType::Library {
                find_opposition_agent(ctx, controller).unwrap_or(chooser)
            } else {
                chooser
            };

            if is_optional {
                let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
                let origin_label = origin_zone.to_string().to_lowercase();
                let message = format!(
                    "Search {}'s {}?",
                    ctx.game.player(affected_player).name,
                    origin_label
                );
                ctx.agents[effective_chooser.index()].snapshot_state(ctx.game, ctx.mana_pools);
                let accepted = ctx.agents[effective_chooser.index()].confirm_action(
                    effective_chooser,
                    Some("ChangeZoneGeneral"),
                    &message,
                    &[],
                    source_name,
                    Some(crate::ability::api_type::ApiType::ChangeZone),
                );
                if !accepted {
                    continue;
                }
            }

            let mut zone_cards = ctx
                .game
                .cards_in_zone(origin_zone, affected_player)
                .to_vec();
            if origin_zone == ZoneType::Library {
                if let Some(max) = find_search_limit(ctx, affected_player, controller) {
                    zone_cards.truncate(max);
                }
            }

            let mut cards_to_move = if let Some(each_spec) = change_type.strip_prefix("EACH ") {
                resolve_each_search(
                    ctx,
                    sa,
                    each_spec,
                    &mut zone_cards,
                    effective_chooser,
                    is_optional,
                )
            } else {
                let candidates: Vec<_> = zone_cards
                    .iter()
                    .copied()
                    .filter(|&cid| matches_with_context(ctx, sa, cid, &change_type))
                    .collect();
                if candidates.is_empty() {
                    Vec::new()
                } else if sa.is_at_random() {
                    resolve_random_selection(ctx, &candidates, change_num)
                } else if change_num == 1 {
                    resolve_single_search(ctx, sa, &candidates, effective_chooser, is_optional)
                } else {
                    resolve_multi_search(
                        ctx,
                        sa,
                        &candidates,
                        effective_chooser,
                        change_num,
                        is_optional,
                    )
                }
            };

            if sa.ir.reorder && cards_to_move.len() > 1 {
                ctx.agents[effective_chooser.index()].snapshot_state(ctx.game, ctx.mana_pools);
                ctx.agents[effective_chooser.index()].on_library_peek(ctx.game, &cards_to_move);
                let reordered = ctx.agents[effective_chooser.index()]
                    .choose_reorder_library(effective_chooser, &cards_to_move);
                if reordered.len() == cards_to_move.len()
                    && cards_to_move.iter().all(|id| reordered.contains(id))
                {
                    cards_to_move = reordered;
                }
            }

            move_cards(
                ctx,
                sa,
                &cards_to_move,
                origin_zone,
                dest_zone,
                &lib_position,
                affected_player,
            );
        }
        return;
    }

    let search_player = if sa.defined_player().is_none() {
        sa.target_chosen.target_player.unwrap_or_else(|| {
            if defined.eq_ignore_ascii_case("Opponent") {
                ctx.game.opponent_of(controller)
            } else {
                controller
            }
        })
    } else if defined.eq_ignore_ascii_case("Opponent") {
        ctx.game.opponent_of(controller)
    } else {
        controller
    };

    // Leonin Arbiter check
    if origin_zone == ZoneType::Library && !can_search_library(ctx, controller) {
        ctx.game
            .shuffle_zone_cards(ZoneType::Library, search_player, ctx.rng);
        return;
    }

    // Opposition Agent — opponent controls the search
    let effective_chooser = if origin_zone == ZoneType::Library {
        find_opposition_agent(ctx, controller).unwrap_or(chooser)
    } else {
        chooser
    };

    if is_optional {
        let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
        let origin_label = origin_zone.to_string().to_lowercase();
        let message = if is_defined {
            format!(
                "Put that card from {}'s {} to {}?",
                ctx.game.player(search_player).name,
                origin_label,
                dest_zone.to_string().to_lowercase()
            )
        } else {
            format!(
                "Search {}'s {}?",
                ctx.game.player(search_player).name,
                origin_label
            )
        };
        ctx.agents[effective_chooser.index()].snapshot_state(ctx.game, ctx.mana_pools);
        let accepted = ctx.agents[effective_chooser.index()].confirm_action(
            effective_chooser,
            Some("ChangeZoneGeneral"),
            &message,
            &[],
            source_name,
            Some(crate::ability::api_type::ApiType::ChangeZone),
        );
        if !accepted {
            return;
        }
    }

    let mut zone_cards = ctx.game.cards_in_zone(origin_zone, search_player).to_vec();

    // Aven Mindcensor restriction
    if origin_zone == ZoneType::Library {
        if let Some(max) = find_search_limit(ctx, search_player, controller) {
            zone_cards.truncate(max);
        }
    }

    // RememberSearched$
    if sa.ir.remember_searched {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).add_remembered_player(search_player);
        }
    }

    // Panglacial Wurm — offer to cast while searching
    if origin_zone == ZoneType::Library {
        offer_panglacial_cast(ctx, sa, controller, &mut zone_cards);
    }

    let mut cards_to_move = if let Some(each_spec) = change_type.strip_prefix("EACH ") {
        resolve_each_search(
            ctx,
            sa,
            each_spec,
            &mut zone_cards,
            effective_chooser,
            is_optional,
        )
    } else {
        let candidates: Vec<_> = zone_cards
            .iter()
            .copied()
            .filter(|&cid| matches_with_context(ctx, sa, cid, &change_type))
            .collect();
        if candidates.is_empty() {
            Vec::new()
        } else if sa.is_at_random() {
            resolve_random_selection(ctx, &candidates, change_num)
        } else if sa.ir.reorder
            && !is_optional
            && origin_zone == ZoneType::Library
            && dest_zone == ZoneType::Library
            && candidates.len() == change_num
        {
            // When the full candidate set must be moved and then reordered,
            // there is no meaningful hidden-zone selection step. Go straight
            // to the reorder callback for the final ordered set.
            candidates
        } else if change_num == 1 {
            resolve_single_search(ctx, sa, &candidates, effective_chooser, is_optional)
        } else {
            resolve_multi_search(
                ctx,
                sa,
                &candidates,
                effective_chooser,
                change_num,
                is_optional,
            )
        }
    };

    if sa.ir.reorder && cards_to_move.len() > 1 {
        ctx.agents[effective_chooser.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[effective_chooser.index()].on_library_peek(ctx.game, &cards_to_move);
        let reordered = ctx.agents[effective_chooser.index()]
            .choose_reorder_library(effective_chooser, &cards_to_move);
        if reordered.len() == cards_to_move.len()
            && cards_to_move.iter().all(|id| reordered.contains(id))
        {
            cards_to_move = reordered;
        }
    }

    // Exactly$ — must find exactly ChangeNum or fail
    if sa.ir.exactly && cards_to_move.len() != change_num {
        if origin_zone == ZoneType::Library {
            ctx.game
                .shuffle_zone_cards(ZoneType::Library, search_player, ctx.rng);
        }
        return;
    }

    // Reveal chosen cards (NoLooking$ suppresses)
    if !sa.ir.no_looking
        && sa.is_reveal()
        && !cards_to_move.is_empty()
        && origin_zone == ZoneType::Library
    {
        for agent in ctx.agents.iter_mut() {
            agent.on_library_peek(ctx.game, &cards_to_move);
        }
    }

    // RememberLKI$
    if sa.ir.remember_lki_flag {
        if let Some(sid) = sa.source {
            for &cid in &cards_to_move {
                ctx.game.card_mut(sid).add_remembered_card(cid);
            }
        }
    }

    move_cards(
        ctx,
        sa,
        &cards_to_move,
        origin_zone,
        dest_zone,
        &lib_position,
        controller,
    );
}

/// Offer Panglacial Wurm cast during library search (CR 702.113).
fn offer_panglacial_cast(
    ctx: &mut EffectContext,
    _sa: &SpellAbility,
    controller: PlayerId,
    zone_cards: &mut Vec<crate::ids::CardId>,
) {
    let panglacial: Vec<_> = zone_cards
        .iter()
        .copied()
        .filter(|&cid| {
            ctx.game
                .card(cid)
                .keywords
                .contains_string_ignore_case("Panglacial")
        })
        .collect();
    for pg_id in panglacial {
        let name = ctx.game.card(pg_id).card_name.clone();
        let cast = ctx.agents[controller.index()].confirm_action(
            controller,
            Some("PanglacialCast"),
            &format!("Cast {} from library while searching?", name),
            &[],
            Some(&name),
            None,
        );
        if cast {
            zone_cards.retain(|&cid| cid != pg_id);
            ctx.move_card(pg_id, ZoneType::Stack, controller);
        }
    }
}
