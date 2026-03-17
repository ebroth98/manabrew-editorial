use forge_foundation::ZoneType;

use super::{
    emit_zone_trigger, evaluate_svar, matches_change_type, parse_counter_type, parse_zone_type,
    EffectContext,
};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let origin_str = sa.origin().unwrap_or("");
    let destination_str = sa.destination().unwrap_or("");
    let tapped = sa.is_tapped();
    let change_type = sa.change_type().unwrap_or("").to_string();
    let defined = sa.defined().unwrap_or("").to_string();
    let lib_position = sa.library_position().unwrap_or("").to_string();
    let shuffle = sa.is_shuffle();
    let remember_changed = sa.is_remember_changed();
    let controller = sa.activating_player;
    let matches_with_context = |card_id, clause: &str| {
        let card = ctx.game.card(card_id);
        if !matches_change_type(card, clause, &[]) {
            return false;
        }
        // Java ChangeType supports numeric CMC comparators such as "cmcLE3" and
        // variable-referenced forms like "cmcLEX" (resolved through SVars).
        for qualifier in clause.split('.').skip(1) {
            if let Some(raw_max) = qualifier.strip_prefix("cmcLE") {
                let max_cmc = if let Ok(v) = raw_max.parse::<i32>() {
                    v
                } else if raw_max.eq_ignore_ascii_case("X") {
                    if let Some(source_id) = sa.source {
                        if let Some(expr) = ctx.game.card(source_id).svars.get("X") {
                            evaluate_svar(expr, sa)
                        } else {
                            sa.x_mana_cost_paid as i32
                        }
                    } else {
                        sa.x_mana_cost_paid as i32
                    }
                } else if let Some(source_id) = sa.source {
                    if let Some(expr) = ctx.game.card(source_id).svars.get(raw_max) {
                        evaluate_svar(expr, sa)
                    } else {
                        return false;
                    }
                } else {
                    return false;
                };
                if card.mana_cost.cmc() as i32 > max_cmc {
                    return false;
                }
            }
        }
        true
    };

    if let (Some(dest_zone), Some(origin_zone)) = (
        parse_zone_type(destination_str),
        parse_zone_type(origin_str),
    ) {
        // Determine which card(s) to move.
        // Only use target_chosen if the SA actually declares targeting (has ValidTgts$).
        // Trigger-inherited targets (e.g. damage_target_card) should NOT be used for
        // library searches — mirrors Java's isHidden()/changeHiddenOriginResolve split.
        let cards_to_move: Vec<_> = if sa.uses_targeting() {
            if let Some(cid) = sa.target_chosen.target_card {
                if ctx.game.card(cid).zone == origin_zone {
                    vec![cid]
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else if defined.eq_ignore_ascii_case("TriggeredNewCardLKICopy")
            || defined.eq_ignore_ascii_case("TriggeredCard")
        {
            // Trigger context card (LKI/new card copy in Forge terms).
            // Common for "when CARDNAME dies, exile it..." style abilities.
            sa.trigger_source
                .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                .into_iter()
                .collect()
        } else if let Some(uid_str) = defined.strip_prefix("CardUID_") {
            // Specific card by ID (e.g. delayed trigger for Dash bounce-to-hand)
            uid_str
                .parse::<u32>()
                .ok()
                .map(crate::ids::CardId)
                .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                .into_iter()
                .collect()
        } else if defined.eq_ignore_ascii_case("Self")
            || (defined.is_empty() && origin_zone.is_known() && sa.defined_player().is_none())
        {
            // Java parity: missing Defined defaults to Self for known-origin ChangeZone;
            // hidden-origin empty Defined uses search flow below.
            // Skip this branch when DefinedPlayer$ is present (e.g. Exhume).
            sa.source
                .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                .into_iter()
                .collect()
        } else if defined.eq_ignore_ascii_case("ExiledWith") {
            // Cards exiled with this source card, tracked via two mechanisms:
            // 1. exiled_by field (set by ChangeZoneAll Duration$ UntilHostLeavesPlay)
            // 2. remembered_cards on the source (set by BeholdExile cost)
            if let Some(source_id) = sa.source {
                let mut result: Vec<_> = ctx
                    .game
                    .cards
                    .iter()
                    .filter(|c| c.zone == origin_zone && c.exiled_by == Some(source_id))
                    .map(|c| c.id)
                    .collect();
                // Also check source's remembered_cards for BeholdExile tracking
                let remembered: Vec<_> = ctx
                    .game
                    .card(source_id)
                    .remembered_cards
                    .iter()
                    .copied()
                    .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                    .filter(|cid| !result.contains(cid))
                    .collect();
                result.extend(remembered);
                result
            } else {
                Vec::new()
            }
        } else if defined.eq_ignore_ascii_case("Remembered") {
            if let Some(source_id) = sa.source {
                ctx.game
                    .card(source_id)
                    .remembered_cards
                    .iter()
                    .copied()
                    .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                    .collect()
            } else {
                Vec::new()
            }
        } else if sa.defined_player().is_some() {
            // DefinedPlayer$ Player — each defined player independently chooses and moves
            // a card from their own zone. Used by Exhume, etc.
            // Mirrors Java's changeHiddenOriginResolve → changeZonePlayerInvariant loop.
            let defined_player = sa.defined_player().unwrap_or("");
            let players: Vec<crate::ids::PlayerId> =
                if defined_player.eq_ignore_ascii_case("Player") {
                    // "Player" = all players in turn order
                    (0..ctx.game.players.len())
                        .map(|i| crate::ids::PlayerId(i as u32))
                        .collect()
                } else if defined_player.eq_ignore_ascii_case("You") {
                    vec![controller]
                } else if defined_player.eq_ignore_ascii_case("Opponent") {
                    vec![ctx.game.opponent_of(controller)]
                } else {
                    vec![controller]
                };
            let mut collected = Vec::new();
            for pid in players {
                let zone_cards = ctx.game.cards_in_zone(origin_zone, pid).to_vec();
                let candidates: Vec<_> = zone_cards
                    .into_iter()
                    .filter(|&cid| matches_with_context(cid, &change_type))
                    .collect();
                if candidates.is_empty() {
                    continue;
                }
                ctx.agents[pid.index()].snapshot_state(ctx.game, ctx.mana_pools);
                if let Some(chosen) = ctx.agents[pid.index()].choose_single_card_for_zone_change(
                    pid,
                    &candidates,
                    "Select card for zone change",
                    false,
                ) {
                    collected.push(chosen);
                }
            }
            collected
        } else if (defined.is_empty() && (origin_zone.is_hidden() || sa.is_hidden()))
            || defined.eq_ignore_ascii_case("You")
            || defined.eq_ignore_ascii_case("Opponent")
        {
            // Hidden-origin search flow (library/hand) with optional chooser prompt.
            let search_player = if defined.eq_ignore_ascii_case("Opponent") {
                ctx.game.opponent_of(controller)
            } else {
                controller
            };
            let mut zone_cards = ctx.game.cards_in_zone(origin_zone, search_player).to_vec();
            if let Some(each_spec) = change_type.strip_prefix("EACH ") {
                let mut out = Vec::new();
                for clause in each_spec
                    .split('&')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let candidates: Vec<_> = zone_cards
                        .iter()
                        .copied()
                        .filter(|&cid| matches_with_context(cid, clause))
                        .collect();
                    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
                    ctx.agents[controller.index()].on_library_peek(ctx.game, &candidates);
                    if let Some(chosen) = ctx.agents[controller.index()]
                        .choose_single_card_for_zone_change(
                            controller,
                            &candidates,
                            "Select card for zone change",
                            false,
                        )
                    {
                        out.push(chosen);
                        if let Some(pos) = zone_cards.iter().position(|&cid| cid == chosen) {
                            zone_cards.remove(pos);
                        }
                    }
                }
                out
            } else {
                let candidates: Vec<_> = zone_cards
                    .into_iter()
                    .filter(|&cid| matches_with_context(cid, &change_type))
                    .collect();
                ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
                ctx.agents[controller.index()].on_library_peek(ctx.game, &candidates);
                ctx.agents[controller.index()]
                    .choose_single_card_for_zone_change(
                        controller,
                        &candidates,
                        "Select card for zone change",
                        false,
                    )
                    .into_iter()
                    .collect()
            }
        } else {
            Vec::new()
        };

        // Fire SearchedLibrary trigger when searching from library
        if origin_zone == ZoneType::Library && !cards_to_move.is_empty() {
            ctx.trigger_handler.run_trigger(
                TriggerType::SearchedLibrary,
                RunParams {
                    player: Some(controller),
                    ..Default::default()
                },
                false,
            );
        }

        for card_id in cards_to_move {
            if remember_changed {
                if let Some(source_id) = sa.source {
                    ctx.game.card_mut(source_id).add_remembered_card(card_id);
                }
            }
            let card_owner = ctx.game.card(card_id).owner;
            let dest_owner = if dest_zone == ZoneType::Battlefield {
                controller
            } else {
                card_owner
            };
            let old_zone = ctx.game.card(card_id).zone;
            ctx.game.move_card(card_id, dest_zone, dest_owner);

            // Handle library bottom positioning (move_card adds to top by default)
            if dest_zone == ZoneType::Library
                && (lib_position == "-1" || lib_position.eq_ignore_ascii_case("Bottom"))
            {
                let zone = ctx.game.zone_mut(ZoneType::Library, dest_owner);
                if let Some(pos) = zone.cards.iter().rposition(|&c| c == card_id) {
                    zone.cards.remove(pos);
                    zone.cards.insert(0, card_id); // index 0 = bottom
                }
            }

            if dest_zone == ZoneType::Battlefield {
                if tapped {
                    ctx.game.tap(card_id);
                }

                // Ninjutsu: mark the card for combat entry. The actual combat.declare_attacker
                // is handled in magic_stack.rs where CombatState is accessible.
                if sa.param_is_true("Ninjutsu") {
                    let defender_pid = ctx.game.opponent_of(controller);
                    ctx.game.card_mut(card_id).attacking_player = Some(defender_pid);
                }

                // Unearth: grant Haste and clear summoning sickness.
                // The creature should be exiled at EOT or if it would leave the battlefield,
                // but for simplicity we grant haste as a pump keyword (clears at cleanup).
                if sa.param_is_true("Unearth") {
                    ctx.game
                        .card_mut(card_id)
                        .pump_keywords
                        .push("Haste".to_string());
                    ctx.game.card_mut(card_id).summoning_sick = false;
                }

                // WithCountersType$: add a counter when entering the battlefield
                // (e.g. Undying adds P1P1, Persist adds M1M1).
                // Mirrors Java's ChangeZoneEffect "WithCountersType" parameter.
                if let Some(counter_type_str) = sa.with_counters_type() {
                    let ct = parse_counter_type(counter_type_str);
                    let amount = sa.with_counters_amount().unwrap_or(1);
                    ctx.game.card_mut(card_id).add_counter(&ct, amount);
                }

                ctx.trigger_handler
                    .register_active_trigger(ctx.game, card_id);
            }

            // Fire Exiled trigger when a card moves to exile
            if dest_zone == ZoneType::Exile {
                ctx.trigger_handler.run_trigger(
                    TriggerType::Exiled,
                    RunParams {
                        card: Some(card_id),
                        origin: Some(old_zone),
                        destination: Some(dest_zone),
                        ..Default::default()
                    },
                    false,
                );
            }

            emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, dest_zone);
        }

        // Shuffle the library after a search (when origin was Library)
        if (origin_zone == ZoneType::Library || shuffle) && dest_zone != ZoneType::Library {
            let lib_cards = ctx
                .game
                .cards_in_zone(ZoneType::Library, controller)
                .to_vec();
            if !lib_cards.is_empty() {
                let lib = ctx.game.zone_mut(ZoneType::Library, controller);
                ctx.rng.shuffle_cards(&mut lib.cards);
                ctx.trigger_handler.run_trigger(
                    TriggerType::Shuffled,
                    RunParams {
                        player: Some(controller),
                        ..Default::default()
                    },
                    false,
                );
            }
        }
    }
}
