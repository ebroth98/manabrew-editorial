use forge_foundation::ZoneType;

use super::{matches_valid_cards, resolve_defined_players, EffectContext};
use crate::agent::BinaryChoiceKind;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

/// `SP$ TapOrUntapAll` — choose tap or untap, then apply to all matching cards.
///
/// Mirrors Java `TapOrUntapAllEffect.java` binary prompt behavior.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let source_name = sa
        .source
        .map(|cid| ctx.game.card(cid).card_name.clone())
        .unwrap_or_else(|| "Ability".to_string());

    let prompt = "Tap or untap all matching permanents?";
    let to_tap = ctx.agents[controller.index()].choose_binary(
        controller,
        prompt,
        BinaryChoiceKind::TapOrUntap,
        None,
        Some(&source_name),
        sa.api.as_deref(),
    );

    let valid_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Permanent".to_string());

    let restrict_controllers: Option<Vec<PlayerId>> =
        if let Some(pid) = sa.target_chosen.target_player {
            Some(vec![pid])
        } else if let Some(defined) = sa.params.get("Defined") {
            Some(resolve_defined_players(defined, controller, ctx.game))
        } else {
            None
        };

    let mut cards: Vec<CardId> = Vec::new();
    for &pid in &ctx.game.player_order {
        for cid in ctx
            .game
            .cards_in_zone(ZoneType::Battlefield, pid)
            .iter()
            .copied()
        {
            if let Some(ref allowed) = restrict_controllers {
                if !allowed.contains(&ctx.game.card(cid).controller) {
                    continue;
                }
            }
            if matches_valid_cards(ctx.game.card(cid), &valid_filter, controller) {
                cards.push(cid);
            }
        }
    }

    for cid in cards {
        if ctx.game.card(cid).zone != ZoneType::Battlefield {
            continue;
        }
        if to_tap {
            if ctx.game.tap(cid) {
                ctx.trigger_handler.run_trigger(
                    crate::event::TriggerType::Taps,
                    crate::event::RunParams {
                        card: Some(cid),
                        player: Some(controller),
                        ..Default::default()
                    },
                    false,
                );
            }
        } else if ctx.game.untap(cid) {
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::Untaps,
                crate::event::RunParams {
                    card: Some(cid),
                    player: Some(controller),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
