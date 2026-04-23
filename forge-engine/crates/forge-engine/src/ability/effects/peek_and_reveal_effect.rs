use forge_foundation::ZoneType;

use super::{resolve_numeric_svar, EffectContext};
use crate::spellability::SpellAbility;

/// Mirrors Java's `PeekAndRevealEffect.java`.
///
/// `DB$ PeekAndReveal | NumCards$ N | RememberRevealed$ True`
///
/// Peeks at the top N cards of the controller's library without removing them.
/// If `RememberRevealed$ True`, stores the peeked card IDs on the source card's
/// `remembered_cards` list so that a subsequent `SetState | Mode$ Transform`
/// condition check can inspect them.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `PeekAndRevealEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(PeekAndRevealEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let num = if sa.params.has("PeekAmount") {
        resolve_numeric_svar(ctx.game, sa, "PeekAmount", 1)
    } else {
        resolve_numeric_svar(ctx.game, sa, "NumCards", 1)
    }
    .max(0) as usize;
    let remember_revealed = sa
        .params
        .get("RememberPeeked")
        .or(sa.params.get("RememberRevealed"))
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));
    let remember_peeked = sa
        .params
        .get("RememberPeeked")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    let controller = sa.activating_player;
    let no_peek = sa
        .params
        .get("NoPeek")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    // Collect the top N card IDs from the library (last N entries = top of library).
    let peeked: Vec<_> = {
        let lib = ctx.game.cards_in_zone(ZoneType::Library, controller);
        let start = lib.len().saturating_sub(num);
        lib[start..].to_vec()
    };

    if !no_peek && !peeked.is_empty() {
        ctx.agents[controller.index()].on_library_peek(ctx.game, &peeked);
        ctx.agents[controller.index()].reveal_cards(
            ctx.game,
            controller,
            &peeked,
            ZoneType::Library,
            controller,
            sa.source.map(|cid| ctx.game.card(cid).card_name.as_str()),
        );
    }

    if remember_revealed || remember_peeked {
        if let Some(source_id) = sa.source {
            for &card_id in &peeked {
                ctx.game.card_mut(source_id).add_remembered_card(card_id);
            }
        }
    }
}
