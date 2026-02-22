use forge_foundation::ZoneType;

use super::{parse_param, EffectContext};
use crate::spellability::SpellAbility;

/// Mirrors Java's `PeekAndRevealEffect.java`.
///
/// `DB$ PeekAndReveal | NumCards$ N | RememberRevealed$ True`
///
/// Peeks at the top N cards of the controller's library without removing them.
/// If `RememberRevealed$ True`, stores the peeked card IDs on the source card's
/// `remembered_cards` list so that a subsequent `SetState | Mode$ Transform`
/// condition check can inspect them.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1) as usize;
    let remember = sa
        .params
        .get("RememberRevealed")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));

    let controller = sa.activating_player;

    // Collect the top N card IDs from the library (last N entries = top of library).
    let peeked: Vec<_> = {
        let lib = ctx.game.cards_in_zone(ZoneType::Library, controller);
        let start = lib.len().saturating_sub(num);
        lib[start..].to_vec()
    };

    if remember {
        if let Some(source_id) = sa.source {
            for &card_id in &peeked {
                ctx.game.card_mut(source_id).add_remembered_card(card_id);
            }
        }
    }
}
