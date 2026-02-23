use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, parse_zone_type, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let origin_str = sa.params.get("Origin").map(|s| s.as_str()).unwrap_or("");
    let destination_str = sa
        .params
        .get("Destination")
        .map(|s| s.as_str())
        .unwrap_or("");
    let tapped = sa
        .params
        .get("Tapped")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);
    let change_type = sa.params.get("ChangeType").cloned().unwrap_or_default();
    let defined = sa.params.get("Defined").cloned().unwrap_or_default();
    let lib_position = sa
        .params
        .get("LibraryPosition")
        .cloned()
        .unwrap_or_default();
    let shuffle = sa
        .params
        .get("Shuffle")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);
    let controller = sa.activating_player;

    if let (Some(dest_zone), Some(origin_zone)) = (
        parse_zone_type(destination_str),
        parse_zone_type(origin_str),
    ) {
        // Determine which card to move
        let card_to_move = if let Some(cid) = sa.target_chosen.target_card {
            // Targeted effect: move if it's in the expected origin zone
            if ctx.game.card(cid).zone == origin_zone {
                Some(cid)
            } else {
                None
            }
        } else if defined.eq_ignore_ascii_case("Self") {
            // Move the source card itself
            sa.source
                .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
        } else if defined.is_empty() || defined.eq_ignore_ascii_case("You") {
            // No target: search controller's origin zone for a matching card (e.g. library tutor)
            let search_player = if defined.eq_ignore_ascii_case("Opponent") {
                ctx.game.opponent_of(controller)
            } else {
                controller
            };
            let zone_cards = ctx.game.cards_in_zone(origin_zone, search_player).to_vec();
            zone_cards
                .into_iter()
                .find(|&cid| matches_change_type(ctx.game.card(cid), &change_type))
        } else {
            None
        };

        if let Some(card_id) = card_to_move {
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
                // No RNG available here — reverse as a placeholder shuffle
                let zone = ctx.game.zone_mut(ZoneType::Library, controller);
                zone.cards.reverse();
            }
        }
    }
}
