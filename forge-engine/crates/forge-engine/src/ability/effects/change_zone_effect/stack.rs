//! Stack removal — bouncing/exiling spells on the stack.
//!
//! Mirrors Java's `removeFromStack` (lines 1593-1649).

use forge_foundation::ZoneType;

use super::helpers::apply_post_move;
use super::super::{emit_zone_trigger, parse_counter_type, EffectContext};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

/// Remove a spell from the stack and move it to a destination zone.
pub(super) fn resolve_stack_removal(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    dest_zone: ZoneType,
    lib_position: &str,
    _controller: PlayerId,
) {
    let target_card = if sa.uses_targeting() {
        sa.target_chosen.target_card
    } else {
        sa.trigger_source.filter(|&cid| ctx.game.card(cid).zone == ZoneType::Stack)
    };

    let Some(card_id) = target_card else { return };
    if ctx.game.card(card_id).zone != ZoneType::Stack { return; }

    // Tokens on stack cease to exist when exiled
    if dest_zone == ZoneType::Exile && ctx.game.card(card_id).is_token { return; }

    let old_zone = ctx.game.card(card_id).zone;
    let dest_owner = ctx.game.card(card_id).owner;
    ctx.game.move_card(card_id, dest_zone, dest_owner);

    // ExiledWith for exile
    if dest_zone == ZoneType::Exile {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(card_id).exiled_by = Some(source_id);
        }
    }

    // Library positioning
    if dest_zone == ZoneType::Library
        && (lib_position == "-1" || lib_position.eq_ignore_ascii_case("Bottom"))
    {
        let zone = ctx.game.zone_mut(ZoneType::Library, dest_owner);
        if let Some(pos) = zone.cards.iter().rposition(|&c| c == card_id) {
            zone.cards.remove(pos);
            zone.cards.insert(0, card_id);
        }
    }

    if dest_zone == ZoneType::Library && sa.is_shuffle() {
        let lib = ctx.game.zone_mut(ZoneType::Library, dest_owner);
        ctx.rng.shuffle_cards(&mut lib.cards);
    }

    // Counters
    if let Some(ct_str) = sa.with_counters_type() {
        let ct = parse_counter_type(ct_str);
        ctx.game.card_mut(card_id).add_counter(&ct, sa.with_counters_amount().unwrap_or(1));
    }

    // Remember/Imprint
    if sa.is_remember_changed() {
        if let Some(sid) = sa.source { ctx.game.card_mut(sid).add_remembered_card(card_id); }
    }
    if sa.is_imprint() {
        if let Some(sid) = sa.source { ctx.game.card_mut(sid).imprinted_cards.push(card_id); }
    }

    emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, dest_zone);
}
