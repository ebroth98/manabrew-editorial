use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let sac_valid = sa
        .params
        .get("SacValid")
        .cloned()
        .unwrap_or_else(|| "Self".to_string());
    let defined = sa
        .params
        .get("Defined")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // "Defined$ Player" means each player sacrifices (e.g. Innocent Blood).
    // "ValidTgts$ Player" means a targeted player sacrifices (e.g. Diabolic Edict) —
    // in that case sa.target_chosen.target_player is set. Otherwise default to the controller.
    let sacrificing_players: Vec<PlayerId> = if defined == "player" {
        ctx.game.player_order.clone()
    } else {
        vec![sa
            .target_chosen
            .target_player
            .unwrap_or(sa.activating_player)]
    };

    for sacrificing_player in sacrificing_players {
        let card_to_sacrifice = if sac_valid.eq_ignore_ascii_case("Self") {
            // Sacrifice the source card itself
            sa.source
                .filter(|&cid| ctx.game.card(cid).zone == ZoneType::Battlefield)
        } else {
            // Find valid cards controlled by the sacrificing player
            let valid: Vec<_> = ctx
                .game
                .cards_in_zone(ZoneType::Battlefield, sacrificing_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| matches_change_type(ctx.game.card(cid), &sac_valid))
                .collect();

            if valid.is_empty() {
                None
            } else {
                ctx.agents[sacrificing_player.index()].choose_sacrifice(sacrificing_player, &valid)
            }
        };

        if let Some(card_id) = card_to_sacrifice {
            if ctx.game.card(card_id).zone == ZoneType::Battlefield {
                let owner = ctx.game.card(card_id).owner;
                // Fire Sacrificed trigger
                ctx.trigger_handler.run_trigger(
                    TriggerType::Sacrificed,
                    RunParams {
                        card: Some(card_id),
                        player: Some(sacrificing_player),
                        ..Default::default()
                    },
                    false,
                );
                ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
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
