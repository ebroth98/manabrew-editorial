//! Shared helpers for the ChangeZone effect module.
//!
//! Contains: card matching, pre/post move logic, destination resolution,
//! search restrictions, and effect creation.

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use super::super::{
    emit_zone_trigger, evaluate_svar, matches_change_type, parse_counter_type, parse_zone_type,
    resolve_defined_players, EffectContext,
};
use crate::card::CardInstance;
use crate::event::{RunParams, TriggerType};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;
use crate::staticability::parse_static_ability;

// ─── Card Matching ──────────────────────────────────────────────────────────

/// Check if a card matches a ChangeType clause including CMC qualifiers.
pub(super) fn matches_with_context(
    ctx: &EffectContext,
    sa: &SpellAbility,
    card_id: CardId,
    clause: &str,
) -> bool {
    let card = ctx.game.card(card_id);
    if !matches_change_type(card, clause, &[]) {
        return false;
    }
    for qualifier in clause.split('.').skip(1) {
        if let Some(raw_max) = qualifier.strip_prefix("cmcLE") {
            let max_cmc = if let Ok(v) = raw_max.parse::<i32>() {
                v
            } else if raw_max.eq_ignore_ascii_case("X") {
                sa.source
                    .and_then(|sid| ctx.game.card(sid).svars.get("X").map(|e| evaluate_svar(e, sa)))
                    .unwrap_or(sa.x_mana_cost_paid as i32)
            } else {
                match sa.source.and_then(|sid| ctx.game.card(sid).svars.get(raw_max).map(|e| evaluate_svar(e, sa))) {
                    Some(v) => v,
                    None => return false,
                }
            };
            if card.mana_cost.cmc() as i32 > max_cmc {
                return false;
            }
        }
    }
    true
}

/// Check if all candidates are fungible (same card name).
/// NOTE: Not used for search parity — Java always delegates to the player
/// controller even for fungible candidates, so we must do the same to keep
/// agent RNG consumption in sync.
#[allow(dead_code)]
pub(super) fn all_candidates_fungible(ctx: &EffectContext, candidates: &[CardId]) -> bool {
    if candidates.len() <= 1 {
        return true;
    }
    let first_name = &ctx.game.card(candidates[0]).card_name;
    candidates[1..]
        .iter()
        .all(|&cid| ctx.game.card(cid).card_name == *first_name)
}

/// Collect all card IDs currently on the battlefield.
pub(super) fn battlefield_card_ids(ctx: &EffectContext) -> Vec<CardId> {
    ctx.game.cards.iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .map(|c| c.id)
        .collect()
}

// ─── Land Type Utilities ────────────────────────────────────────────────────

const BASIC_LAND_TYPES: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// Extract basic land subtypes from a card's subtypes list.
pub(super) fn get_land_subtypes(subtypes: &[String]) -> Vec<String> {
    subtypes.iter()
        .filter(|s| BASIC_LAND_TYPES.iter().any(|blt| s.eq_ignore_ascii_case(blt)))
        .cloned()
        .collect()
}

// ─── Search Restrictions ────────────────────────────────────────────────────

/// Check for Aven Mindcensor — limits search to top N cards.
pub(super) fn find_search_limit(ctx: &EffectContext, _search_player: PlayerId, searcher: PlayerId) -> Option<usize> {
    for card in ctx.game.cards.iter() {
        if card.zone != ZoneType::Battlefield || card.controller == searcher { continue; }
        for kw in &card.keywords {
            if let Some(rest) = kw.strip_prefix("LimitSearchLibrary:") {
                if let Ok(n) = rest.trim().parse::<usize>() { return Some(n); }
            }
        }
    }
    None
}

/// Check for Opposition Agent — redirects search control to an opponent.
pub(super) fn find_opposition_agent(ctx: &EffectContext, searcher: PlayerId) -> Option<PlayerId> {
    for card in ctx.game.cards.iter() {
        if card.zone != ZoneType::Battlefield || card.controller == searcher { continue; }
        for kw in &card.keywords {
            if kw.eq_ignore_ascii_case("OppositionAgent") || kw.contains("ControlSearching") {
                return Some(card.controller);
            }
        }
        if card.card_name == "Opposition Agent" { return Some(card.controller); }
    }
    None
}

/// Check if a player can search their library (Leonin Arbiter, etc.)
pub(super) fn can_search_library(ctx: &EffectContext, searcher: PlayerId) -> bool {
    for card in ctx.game.cards.iter() {
        if card.zone != ZoneType::Battlefield { continue; }
        for kw in &card.keywords {
            if kw.eq_ignore_ascii_case("CantSearchLibrary") { return false; }
            if kw.starts_with("CantSearchLibraryUnlessPaid") && card.controller != searcher { return false; }
        }
    }
    true
}

// ─── Destination Resolution ─────────────────────────────────────────────────

/// Handle DestinationAlternative$ — player chooses between two destinations.
pub(super) fn resolve_destination(
    ctx: &mut EffectContext, sa: &SpellAbility, dest_zone: ZoneType,
) -> (ZoneType, String) {
    let lib_position = sa.library_position().unwrap_or("").to_string();
    if let Some(alt_dest_str) = sa.destination_alternative() {
        if let Some(alt_zone) = parse_zone_type(alt_dest_str) {
            let alt_lib_pos = sa.params.get("LibraryPositionAlternative")
                .map(|s| s.as_str()).unwrap_or("0").to_string();
            let chooser = sa.activating_player;
            ctx.agents[chooser.index()].snapshot_state(ctx.game, ctx.mana_pools);
            let options = vec![format!("{:?}", dest_zone), format!("{:?}", alt_zone)];
            let use_alt = ctx.agents[chooser.index()].confirm_action(
                chooser, Some("ChangeZoneToAltDestination"), "Choose destination",
                &options, None, None,
            );
            return if use_alt { (alt_zone, alt_lib_pos) } else { (dest_zone, lib_position) };
        }
    }
    (dest_zone, lib_position)
}

/// Determine the controller/owner for the destination zone.
pub(super) fn resolve_dest_owner(
    ctx: &EffectContext, sa: &SpellAbility, card_id: CardId, dest_zone: ZoneType,
) -> PlayerId {
    if dest_zone == ZoneType::Battlefield && sa.is_gain_control() {
        sa.activating_player
    } else {
        ctx.game.card(card_id).owner
    }
}

// ─── Pre/Post Move Logic ────────────────────────────────────────────────────

/// Apply pre-move effects. Returns false if the card should NOT be moved.
pub(super) fn apply_pre_move(
    ctx: &mut EffectContext, card_id: CardId, sa: &SpellAbility, dest_zone: ZoneType,
) -> bool {
    // canExiledBy check
    if dest_zone == ZoneType::Exile {
        if ctx.game.card(card_id).keywords.iter().any(|k| k.eq_ignore_ascii_case("CantBeExiled")) {
            return false;
        }
    }

    if dest_zone == ZoneType::Battlefield {
        // FaceDown$ — before move
        if sa.is_face_down() { ctx.game.card_mut(card_id).face_down = true; }

        // Transformed$ — before move
        if sa.is_transformed() {
            if ctx.game.card(card_id).other_part.is_some() {
                ctx.game.card_mut(card_id).is_transformed = true;
                if let Some(ref other) = ctx.game.card(card_id).other_part {
                    ctx.game.card_mut(card_id).card_name = other.name.clone();
                }
            } else {
                return false;
            }
        }

        // AttachedTo$ — choose and attach before ETB
        if let Some(attached_to_def) = sa.attached_to() {
            let valid: Vec<CardId> = battlefield_card_ids(ctx).into_iter()
                .filter(|&cid| matches_change_type(ctx.game.card(cid), attached_to_def, &[]))
                .collect();
            if !valid.is_empty() {
                let ctrl = sa.activating_player;
                ctx.agents[ctrl.index()].snapshot_state(ctx.game, ctx.mana_pools);
                if let Some(target) = ctx.agents[ctrl.index()].choose_single_card_for_zone_change(
                    ctrl, &valid, "Select a card to attach to", false,
                ) {
                    ctx.game.card_mut(card_id).attached_to = Some(target);
                    ctx.game.card_mut(target).attachments.push(card_id);
                }
            } else if ctx.game.card(card_id).type_line.subtypes.iter().any(|s| s.eq_ignore_ascii_case("Aura")) {
                return false;
            }
        }

        // AttachedToPlayer$ — Curses
        if let Some(atp_def) = sa.params.get("AttachedToPlayer") {
            let players = resolve_defined_players(atp_def, sa.activating_player, ctx.game);
            if players.is_empty() { return false; }
        }
    }

    true
}

/// Apply shared post-move logic for a card entering a destination zone.
pub(super) fn apply_post_move(
    ctx: &mut EffectContext, card_id: CardId, sa: &SpellAbility,
    old_zone: ZoneType, dest_zone: ZoneType, dest_owner: PlayerId, lib_position: &str,
) {
    let controller = sa.activating_player;

    // Remember / Forget / Imprint
    if sa.is_remember_changed() {
        if let Some(sid) = sa.source { ctx.game.card_mut(sid).add_remembered_card(card_id); }
    }
    if sa.is_forget_changed() {
        if let Some(sid) = sa.source { ctx.game.card_mut(sid).remembered_cards.retain(|&c| c != card_id); }
    }
    if sa.is_imprint() {
        if let Some(sid) = sa.source {
            let cm = ctx.game.card_mut(sid);
            if sa.param_is_true("ImprintLast") { cm.imprinted_cards.clear(); }
            cm.imprinted_cards.push(card_id);
        }
    }

    // Library bottom positioning
    if dest_zone == ZoneType::Library && (lib_position == "-1" || lib_position.eq_ignore_ascii_case("Bottom")) {
        let zone = ctx.game.zone_mut(ZoneType::Library, dest_owner);
        if let Some(pos) = zone.cards.iter().rposition(|&c| c == card_id) {
            zone.cards.remove(pos);
            zone.cards.insert(0, card_id);
        }
    }

    // Battlefield entry effects
    if dest_zone == ZoneType::Battlefield {
        if sa.is_tapped() { ctx.game.tap(card_id); }
        if sa.is_gain_control() { ctx.game.card_mut(card_id).controller = controller; }
        if sa.param_is_true("Ninjutsu") {
            ctx.game.card_mut(card_id).attacking_player = Some(ctx.game.opponent_of(controller));
        }
        if sa.param_is_true("Unearth") {
            ctx.game.card_mut(card_id).pump_keywords.push("Haste".to_string());
            ctx.game.card_mut(card_id).summoning_sick = false;
            ctx.game.card_mut(card_id).unearthed = true;
            ctx.trigger_handler.register_delayed_trigger(crate::trigger::handler::DelayedTrigger {
                mode: TriggerType::Phase, trigger_mode: crate::trigger::TriggerMode::Always,
                execute_svar: "UneartheExileDelayedTrigger".to_string(),
                controller, source_card: card_id, target_card: Some(card_id), remembered_amount: 0,
            });
        }
        if sa.param_is_true("Attacking") {
            ctx.game.card_mut(card_id).attacking_player = Some(ctx.game.opponent_of(controller));
        }
        if let Some(ct_str) = sa.with_counters_type() {
            ctx.game.card_mut(card_id).add_counter(&parse_counter_type(ct_str), sa.with_counters_amount().unwrap_or(1));
        }
        ctx.trigger_handler.register_active_trigger(ctx.game, card_id);

        // AttachAfter$
        if let Some(attach_def) = sa.params.get("AttachAfter") {
            let valid: Vec<CardId> = battlefield_card_ids(ctx).into_iter()
                .filter(|&cid| cid != card_id && matches_change_type(ctx.game.card(cid), attach_def, &[]))
                .collect();
            if !valid.is_empty() {
                ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
                if let Some(t) = ctx.agents[controller.index()].choose_single_card_for_zone_change(
                    controller, &valid, "Select a card to attach to", false,
                ) {
                    ctx.game.card_mut(card_id).attached_to = Some(t);
                    ctx.game.card_mut(t).attachments.push(card_id);
                }
            }
        }
    }

    // Exile effects
    if dest_zone == ZoneType::Exile {
        if sa.is_exile_face_down() { ctx.game.card_mut(card_id).face_down = true; }
        if !ctx.game.card(card_id).is_token {
            if let Some(sid) = sa.source {
                // Only set exiled_by when the exile has a Duration$ that returns the card
                // when the host leaves. Permanent exile (like Stalking Leonin) should NOT
                // set exiled_by, otherwise the SBA code will incorrectly return the card
                // when the source leaves play.
                let has_return_duration = sa.params.get("Duration").map_or(false, |d|
                    d.eq_ignore_ascii_case("UntilHostLeavesPlay")
                    || d.eq_ignore_ascii_case("UntilHostLeavesPlayOrEOT")
                    || d.eq_ignore_ascii_case("UntilYourNextTurn")
                );
                if has_return_duration {
                    ctx.game.card_mut(card_id).exiled_by = Some(sid);
                }
                let src_zone = ctx.game.card(sid).zone;
                if matches!(src_zone, ZoneType::Battlefield | ZoneType::Stack | ZoneType::Command) {
                    if !ctx.game.card(sid).remembered_cards.contains(&card_id) {
                        ctx.game.card_mut(sid).add_remembered_card(card_id);
                    }
                }
            }
        }
        ctx.trigger_handler.run_trigger(TriggerType::Exiled, RunParams {
            card: Some(card_id), origin: Some(old_zone), destination: Some(dest_zone), ..Default::default()
        }, false);

        if sa.param_is_true("Foretold") {
            ctx.game.card_mut(card_id).foretold = true;
            if sa.param_is_true("ForetoldCost") { ctx.game.card_mut(card_id).foretold_cost_by_effect = true; }
        }

        // Warp keyword
        let is_warp = sa.params.contains_key("Warp")
            || (sa.trigger_source.is_some() && ctx.game.card(card_id).keywords.iter().any(|k| k.eq_ignore_ascii_case("Warp")));
        if is_warp { create_warp_effect(ctx, sa, card_id); }
    }

    if sa.param_is_true("TrackDiscarded") { ctx.game.card_mut(card_id).discarded = true; }

    // Champion$
    if sa.param_is_true("Champion") {
        ctx.trigger_handler.run_trigger(TriggerType::ChangesZone, RunParams {
            card: Some(card_id), origin: Some(old_zone), destination: Some(dest_zone),
            player: Some(controller), ..Default::default()
        }, false);
    }

    // WithNotedCounters$
    if sa.param_is_true("WithNotedCounters") {
        if let Some(sid) = sa.source {
            let noted = ctx.game.card(sid).remembered_cmc.clone();
            let amount: i32 = noted.iter().sum();
            if amount > 0 {
                let ct = sa.with_counters_type().map(parse_counter_type).unwrap_or_else(|| parse_counter_type("P1P1"));
                ctx.game.card_mut(card_id).add_counter(&ct, amount);
            }
        }
    }

    emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, dest_zone);
}

// ─── Warp Effect ────────────────────────────────────────────────────────────

fn create_warp_effect(ctx: &mut EffectContext, sa: &SpellAbility, exiled_card_id: CardId) {
    let controller = sa.activating_player;
    let card_name = ctx.game.card(exiled_card_id).card_name.clone();
    let mut effect = CardInstance::new(
        CardId(0), format!("Warped {}", card_name), controller,
        CardTypeLine::parse("Effect"), ManaCost::parse("0"), ColorSet::COLORLESS,
        None, None, vec![], vec![],
    );
    effect.controller = controller;
    effect.effect_source = sa.source;
    effect.remembered_cards.push(exiled_card_id);
    effect.forget_on_moved_origin = Some(ZoneType::Exile);
    let static_text = "Mode$ Continuous | MayPlay$ True | EffectZone$ Command | Affected$ Card.IsRemembered+nonLand | AffectedZone$ Exile";
    if let Some(parsed) = parse_static_ability(&format!("S$ {}", static_text)) {
        effect.static_abilities.push(parsed);
    }
    let eid = ctx.game.create_card(effect);
    ctx.game.move_card(eid, ZoneType::Command, controller);
}
