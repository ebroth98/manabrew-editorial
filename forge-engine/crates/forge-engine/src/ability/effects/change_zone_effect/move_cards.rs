//! Shared move + post-processing logic for zone changes.
//!
//! Handles: card ordering, pre/post move, meld, AtEOT, Duration, shuffle.

use forge_foundation::ZoneType;

use super::helpers::{apply_post_move, apply_pre_move, resolve_dest_owner};
use super::super::{emit_zone_trigger, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Move collected cards to destination zone and apply all post-move effects.
pub(super) fn move_cards(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    cards: &[CardId],
    origin_zone: ZoneType,
    dest_zone: ZoneType,
    lib_position: &str,
    controller: PlayerId,
) {
    if cards.is_empty() { return; }

    // SearchedLibrary trigger
    if origin_zone == ZoneType::Library {
        ctx.trigger_handler.run_trigger(TriggerType::SearchedLibrary, RunParams {
            player: Some(controller), ..Default::default()
        }, false);
    }

    // Card ordering for library destination (Java lines 529-539)
    let mut ordered = cards.to_vec();
    if dest_zone == ZoneType::Library && ordered.len() > 1 && !sa.is_shuffle() {
        if sa.param_is_true(keys::RANDOM_ORDER) {
            ctx.rng.shuffle_cards(&mut ordered);
        } else if sa.param_is_true(keys::SHUFFLE_CHANGED_PILE) {
            ctx.rng.shuffle_cards(&mut ordered);
        }
    }

    let mut searched_owners: Vec<PlayerId> = Vec::new();

    // ForgetOtherRemembered$ — clear before processing (Java line 510)
    if sa.param_is_true(keys::FORGET_OTHER_REMEMBERED) {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).clear_remembered();
        }
    }

    let mut moved = Vec::new();

    for &card_id in &ordered {
        if origin_zone == ZoneType::Library {
            let owner = ctx.game.card(card_id).owner;
            if !searched_owners.contains(&owner) { searched_owners.push(owner); }
        }

        if !apply_pre_move(ctx, card_id, sa, dest_zone) { continue; }

        // Collect melded parts before moving (CR 712.4)
        let melded_parts = ctx.game.card(card_id).melded_with.clone();

        let dest_owner = resolve_dest_owner(ctx, sa, card_id, dest_zone);
        let old_zone = ctx.game.card(card_id).zone;
        ctx.game.move_card(card_id, dest_zone, dest_owner);
        apply_post_move(ctx, card_id, sa, old_zone, dest_zone, dest_owner, lib_position);
        moved.push(card_id);

        // Move melded parts together
        for meld_id in melded_parts {
            if ctx.game.card(meld_id).zone == old_zone {
                let mo = ctx.game.card(meld_id).owner;
                let mz = ctx.game.card(meld_id).zone;
                ctx.game.move_card(meld_id, dest_zone, mo);
                emit_zone_trigger(ctx.trigger_handler, meld_id, mz, dest_zone);
                moved.push(meld_id);
            }
        }
    }

    // Searched$ — force trigger even without Library origin
    if sa.param_is_true(keys::SEARCHED) && origin_zone != ZoneType::Library {
        ctx.trigger_handler.run_trigger(TriggerType::SearchedLibrary, RunParams {
            player: Some(controller), ..Default::default()
        }, false);
    }

    // AtEOT$ delayed triggers
    if let Some(eot_svar) = sa.params.get(keys::AT_EOT) {
        for &cid in &moved {
            ctx.trigger_handler.register_delayed_trigger(crate::trigger::handler::DelayedTrigger {
                mode: TriggerType::Phase,
                trigger_mode: crate::trigger::TriggerMode::Always,
                execute_svar: eot_svar.to_string(),
                controller,
                source_card: sa.source.unwrap_or(cid),
                target_card: Some(cid),
                remembered_amount: 0,
            });
        }
    }

    // Duration$ UntilHostLeavesPlay — mark exiled cards for return
    if let Some(duration) = sa.params.get(keys::DURATION) {
        if duration.eq_ignore_ascii_case("UntilHostLeavesPlay")
            || duration.eq_ignore_ascii_case("UntilHostLeavesPlayOrEOT")
        {
            if let Some(sid) = sa.source {
                for &cid in &moved {
                    ctx.game.card_mut(cid).set_exiled_by(Some(sid));
                }
            }
        }
    }

    // Shuffle after library search
    let shuffle_param = sa.params.get(keys::SHUFFLE);
    let no_shuffle = shuffle_param == Some("False") || sa.param_is_true(keys::NO_SHUFFLE);
    let force_shuffle = sa.is_shuffle();
    if !no_shuffle && (origin_zone == ZoneType::Library || force_shuffle) && dest_zone != ZoneType::Library {
        let players = if !searched_owners.is_empty() { searched_owners } else { vec![controller] };
        for pid in players {
            if ctx.game.cards_in_zone(ZoneType::Library, pid).is_empty() { continue; }
            let lib = ctx.game.zone_mut(ZoneType::Library, pid);
            ctx.rng.shuffle_cards(&mut lib.cards);
            ctx.trigger_handler.run_trigger(TriggerType::Shuffled, RunParams {
                player: Some(pid), ..Default::default()
            }, false);
        }
    }
}
