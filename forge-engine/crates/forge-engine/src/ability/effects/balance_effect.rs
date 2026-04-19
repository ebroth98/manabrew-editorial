use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_valid_cards, parse_zone_type, EffectContext};
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::spellability::SpellAbility;

/// `SP$ Balance` — equalize resources across all players.
///
/// Mirrors Java's `BalanceEffect.java`.
///
/// # Params
/// - `Valid` — card filter (default "Card")
/// - `Zone` — zone to balance (default Battlefield)
///
/// Each player with more than the minimum count sacrifices/discards down to the minimum.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let filter = sa
        .params
        .get("Valid")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Card".to_string());

    let zone = sa
        .params
        .get("Zone")
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Battlefield);

    let player_order = ctx.game.player_order.clone();

    // Count valid cards per player
    let counts: Vec<(crate::ids::PlayerId, usize)> = player_order
        .iter()
        .map(|&pid| {
            let count = ctx
                .game
                .cards_in_zone(zone, pid)
                .to_vec()
                .iter()
                .filter(|&&cid| matches_valid_cards(ctx.game.card(cid), &filter, controller))
                .count();
            (pid, count)
        })
        .collect();

    let min_count = counts.iter().map(|(_, c)| *c).min().unwrap_or(0);

    // Each player with excess must sacrifice/discard down
    for &(pid, count) in &counts {
        if count <= min_count {
            continue;
        }
        let excess = count - min_count;

        match zone {
            ZoneType::Battlefield => {
                // Sacrifice excess cards
                for _ in 0..excess {
                    let valid: Vec<_> = ctx
                        .game
                        .cards_in_zone(ZoneType::Battlefield, pid)
                        .to_vec()
                        .into_iter()
                        .filter(|&cid| matches_valid_cards(ctx.game.card(cid), &filter, controller))
                        .collect();

                    if valid.is_empty() {
                        break;
                    }

                    ctx.agents[pid.index()].snapshot_state(ctx.game, ctx.mana_pools);
                    if let Some(card_id) =
                        ctx.agents[pid.index()].choose_sacrifice(pid, &valid, None)
                    {
                        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
                            let owner = ctx.game.card(card_id).owner;
                            ctx.trigger_handler.run_trigger(
                                TriggerType::Sacrificed,
                                RunParams {
                                    card: Some(card_id),
                                    player: Some(pid),
                                    ..Default::default()
                                },
                                false,
                            );
                            ctx.move_card(card_id, ZoneType::Graveyard, owner);
                            emit_zone_trigger(
                                ctx.trigger_handler,
                                card_id,
                                ZoneType::Battlefield,
                                ZoneType::Graveyard,
                            );
                        }
                    }
                }
            }
            ZoneType::Hand => {
                // Discard excess cards
                let hand: Vec<_> = ctx.game.cards_in_zone(ZoneType::Hand, pid).to_vec();
                ctx.agents[pid.index()].snapshot_state(ctx.game, ctx.mana_pools);
                let to_discard = ctx.agents[pid.index()].choose_discard(pid, &hand, excess);

                for &card_id in to_discard.iter().take(excess) {
                    if ctx.game.card(card_id).zone == ZoneType::Hand {
                        ctx.game.player_record_discard(pid, 1);
                        ctx.game.card_mut(card_id).set_discarded(true);
                        let owner = ctx.game.card(card_id).owner;
                        ctx.move_card(card_id, ZoneType::Graveyard, owner);
                        emit_zone_trigger(
                            ctx.trigger_handler,
                            card_id,
                            ZoneType::Hand,
                            ZoneType::Graveyard,
                        );
                        ctx.trigger_handler.run_trigger(
                            TriggerType::Discarded,
                            RunParams {
                                card: Some(card_id),
                                player: Some(pid),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                }
            }
            _ => {
                // Other zones: not commonly balanced, skip
            }
        }
    }
}
