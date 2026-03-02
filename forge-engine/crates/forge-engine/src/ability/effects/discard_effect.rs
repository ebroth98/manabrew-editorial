use forge_foundation::ZoneType;

use super::{emit_zone_trigger, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;

/// SP$ Discard — target player (or defined player) discards N cards.
///
/// Mirrors Java's `DiscardEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let num: usize = sa
        .params
        .get("NumCards")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Determine target player: either from targeting or Defined$
    let target_player: PlayerId = if let Some(pid) = sa.target_chosen.target_player {
        pid
    } else if let Some(defined) = sa.params.get("Defined") {
        match resolve_defined_player(defined, controller, ctx.game) {
            Some(pid) => pid,
            None => return,
        }
    } else {
        controller // default: self
    };

    let hand: Vec<_> = ctx
        .game
        .cards_in_zone(ZoneType::Hand, target_player)
        .to_vec();

    // Mode$ Random — discard at random (e.g. Hypnotic Specter).
    // Mirrors Java's DiscardEffect which calls Aggregates.random() bypassing the controller.
    // We route through the agent's choose_random_discard so deterministic agents can
    // use their seeded RNG for parity testing.
    let is_random = sa
        .params
        .get("Mode")
        .map_or(false, |m| m.eq_ignore_ascii_case("Random"));

    let to_discard = if is_random {
        ctx.agents[target_player.index()].choose_random_discard(target_player, &hand, num)
    } else {
        ctx.agents[target_player.index()].choose_discard(target_player, &hand, num)
    };

    for card_id in to_discard {
        if ctx.game.card(card_id).zone == ZoneType::Hand {
            let owner = ctx.game.card(card_id).owner;
            let has_madness = ctx.game.card(card_id).get_madness_cost().is_some();

            if has_madness {
                // Madness: exile the card instead of putting it into graveyard.
                // The card can then be cast from exile for its madness cost
                // (handled by get_playable_cards checking exile for madness cards).
                ctx.game.move_card(card_id, ZoneType::Exile, owner);
                emit_zone_trigger(
                    ctx.trigger_handler,
                    card_id,
                    ZoneType::Hand,
                    ZoneType::Exile,
                );
                // Mark the card so get_playable_cards can detect it as castable via madness.
                // We use face_down = false to keep it revealed (madness is exile face-up).
                // The actual casting from exile with madness cost is handled by
                // get_playable_cards (checks exile for madness) and play_card (detects madness).
                ctx.game
                    .card_mut(card_id)
                    .granted_keywords
                    .push("MadnessExiled".to_string());
            } else {
                ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
                emit_zone_trigger(
                    ctx.trigger_handler,
                    card_id,
                    ZoneType::Hand,
                    ZoneType::Graveyard,
                );
            }

            // Fire Discarded trigger regardless of destination
            ctx.trigger_handler.run_trigger(
                TriggerType::Discarded,
                RunParams {
                    card: Some(card_id),
                    player: Some(target_player),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
