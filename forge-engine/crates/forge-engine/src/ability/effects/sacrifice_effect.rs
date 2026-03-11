use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;

/// Perform the actual sacrifice of a card: fire triggers, move to graveyard, emit zone change.
/// If `exploit_source` is Some, also fires the Exploited trigger for the Exploit keyword.
fn do_sacrifice(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    card_id: crate::ids::CardId,
    sacrificing_player: PlayerId,
    exploit_source: Option<crate::ids::CardId>,
) {
    if ctx.game.card(card_id).zone != ZoneType::Battlefield {
        return;
    }
    if crate::staticability::static_ability_cant_sacrifice::cant_sacrifice(
        &ctx.game.cards,
        ctx.game.card(card_id),
        Some(sa),
        false,
    ) {
        return;
    }
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
    // Fire Exploited trigger when the sacrifice is from the Exploit keyword
    if let Some(source_id) = exploit_source {
        ctx.trigger_handler.run_trigger(
            TriggerType::Exploited,
            RunParams {
                card: Some(source_id),
                exploited_card: Some(card_id),
                player: Some(sacrificing_player),
                ..Default::default()
            },
            false,
        );
    }
}

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

    // How many permanents to sacrifice (e.g. Annihilator N).
    let amount: usize = sa
        .params
        .get("Amount")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Detect Exploit keyword sacrifice — fires TriggerType::Exploited after each sacrifice.
    let is_exploit = sa.params.get("Exploit").map_or(false, |v| v == "True");
    let exploit_source = if is_exploit { sa.source } else { None };

    let optional = sa.params.contains_key("Optional");
    let is_strict = sa.params.contains_key("StrictAmount");

    // "Defined$ Player" means each player sacrifices (e.g. Innocent Blood).
    // "Defined$ TriggeredDefendingPlayer" means the defending player from an attack trigger.
    // "ValidTgts$ Player" means a targeted player sacrifices (e.g. Diabolic Edict) —
    // in that case sa.target_chosen.target_player is set. Otherwise default to the controller.
    let sacrificing_players: Vec<PlayerId> = if defined == "player" {
        // Match Java getTargetPlayers(): in-game players, ordered in turn
        // order starting with the current turn player (APNAP base order).
        let alive = ctx.game.alive_players();
        let active = ctx.game.active_player();
        let start = alive.iter().position(|&pid| pid == active).unwrap_or(0);
        (0..alive.len())
            .map(|i| alive[(start + i) % alive.len()])
            .collect()
    } else if defined == "triggereddefendingplayer" {
        // Defending player from an attack trigger (e.g. Annihilator).
        // The trigger handler propagates defending_player into target_chosen.target_player.
        vec![sa
            .target_chosen
            .target_player
            .unwrap_or_else(|| ctx.game.opponent_of(sa.activating_player))]
    } else {
        vec![sa
            .target_chosen
            .target_player
            .unwrap_or(sa.activating_player)]
    };

    for sacrificing_player in sacrificing_players {
        if optional {
            let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
            let accepted = ctx.agents[sacrificing_player.index()].confirm_action(
                sacrificing_player,
                None,
                "Do you want to sacrifice?",
                &[],
                source_name,
                Some("Sacrifice"),
            );
            if !accepted {
                continue;
            }
        }

        // When Optional$ True, Java uses choosePermanentsToSacrifice(min=0, max=amount)
        // which allows the player to sacrifice fewer than `amount` creatures.
        // We match this by collecting all chosen cards at once via choose_cards_for_effect.
        if optional && !sac_valid.eq_ignore_ascii_case("Self") && defined.strip_prefix("carduid_").is_none() {
            let valid: Vec<_> = ctx
                .game
                .cards_in_zone(ZoneType::Battlefield, sacrificing_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| matches_change_type(ctx.game.card(cid), &sac_valid, &[]))
                .filter(|&cid| !crate::staticability::static_ability_cant_sacrifice::cant_sacrifice(
                    &ctx.game.cards,
                    ctx.game.card(cid),
                    Some(sa),
                    false,
                ))
                .collect();

            let min_targets = if is_strict { amount } else { 0 };
            let chosen = if valid.is_empty() {
                vec![]
            } else {
                ctx.agents[sacrificing_player.index()]
                    .choose_cards_for_effect(sacrificing_player, &valid, min_targets, amount)
            };

            for card_id in chosen {
                do_sacrifice(ctx, sa, card_id, sacrificing_player, exploit_source);
            }
            continue;
        }

        // Repeat the sacrifice `amount` times (e.g. Annihilator N).
        for _ in 0..amount {
            let card_to_sacrifice = if let Some(uid_str) = defined.strip_prefix("carduid_") {
                // Specific card by ID (e.g. delayed trigger for Blitz sacrifice-at-EOT)
                uid_str
                    .parse::<u32>()
                    .ok()
                    .map(crate::ids::CardId)
                    .filter(|&cid| ctx.game.card(cid).zone == ZoneType::Battlefield)
            } else if sac_valid.eq_ignore_ascii_case("Self") {
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
                    .filter(|&cid| matches_change_type(ctx.game.card(cid), &sac_valid, &[]))
                    .collect();

                if valid.is_empty() {
                    None
                } else {
                    ctx.agents[sacrificing_player.index()]
                        .choose_sacrifice(sacrificing_player, &valid)
                }
            };

            if let Some(card_id) = card_to_sacrifice {
                do_sacrifice(ctx, sa, card_id, sacrificing_player, exploit_source);
            }
        }
    }
}
