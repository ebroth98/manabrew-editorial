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
        // Determine which card(s) to move.
        // Only use target_chosen if the SA actually declares targeting (has ValidTgts$).
        // Trigger-inherited targets (e.g. damage_target_card) should NOT be used for
        // library searches — mirrors Java's isHidden()/changeHiddenOriginResolve split.
        let cards_to_move: Vec<_> =
            if sa.uses_targeting() {
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
        } else if defined.eq_ignore_ascii_case("Self") {
            // Move the source card itself
            sa.source
                .filter(|&cid| ctx.game.card(cid).zone == origin_zone)
                .into_iter()
                .collect()
        } else if defined.is_empty()
            || defined.eq_ignore_ascii_case("You")
            || defined.eq_ignore_ascii_case("Opponent")
        {
            // No target: search selected player's origin zone for a matching card.
            let search_player = if defined.eq_ignore_ascii_case("Opponent") {
                ctx.game.opponent_of(controller)
            } else {
                controller
            };
            let mut zone_cards = ctx.game.cards_in_zone(origin_zone, search_player).to_vec();
            // Sort candidates alphabetically by name to match Java's fetchList.sort().
            // Java sorts before selection so the DeterministicController always picks
            // the first alphabetically, ensuring parity across engines.
            zone_cards.sort_by(|&a, &b| {
                ctx.game.card(a).card_name.cmp(&ctx.game.card(b).card_name)
            });
            if let Some(each_spec) = change_type.strip_prefix("EACH ") {
                let mut out = Vec::new();
                for clause in each_spec.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                    if let Some(pos) = zone_cards
                        .iter()
                        .position(|&cid| matches_change_type(ctx.game.card(cid), clause, &[]))
                    {
                        out.push(zone_cards.remove(pos));
                    }
                }
                out
            } else {
                zone_cards
                    .into_iter()
                    .find(|&cid| matches_change_type(ctx.game.card(cid), &change_type, &[]))
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
