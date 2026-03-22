use forge_foundation::ZoneType;

use super::{emit_zone_trigger_with_lki_counters, matches_change_type, EffectContext};
use crate::card::CounterType;
use crate::event::{RunParams, TriggerType};
use crate::ids::PlayerId;
use crate::parsing::keys;
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

    // Capture +1/+1 counter count BEFORE the card moves to graveyard.
    // Needed for Modular death triggers which move counters to target
    // artifact creature (CR 702.43b). Counters are cleared during move_card.
    let lki_p1p1 = *ctx
        .game
        .card(card_id)
        .counters
        .get(&crate::card::CounterType::P1P1)
        .unwrap_or(&0);

    // Clear temporary Animate triggers before firing events (CR 400.7).
    {
        let card = ctx.game.card_mut(card_id);
        let pt = card.pump_trigger_count;
        if pt > 0 {
            let new_len = card.triggers.len().saturating_sub(pt);
            card.triggers.truncate(new_len);
            card.pump_trigger_count = 0;
        }
    }
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
    // Emit ChangesZone before move so LKI state (counters, keywords)
    // is still available for trigger matching.
    emit_zone_trigger_with_lki_counters(
        ctx.trigger_handler,
        card_id,
        ZoneType::Battlefield,
        ZoneType::Graveyard,
        lki_p1p1,
    );
    ctx.trigger_handler.flush_waiting_triggers(ctx.game);
    ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
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
    // ── Cumulative Upkeep ────────────────────────────────────────────────
    // Mirrors Java SacrificeEffect lines 52-75: when CumulativeUpkeep$ is set,
    // add an Age counter, build merged cost (base cost × age counters),
    // ask player to pay, sacrifice if not paid.
    if let Some(cum_cost_str) = sa.params.get_cloned(keys::CUMULATIVE_UPKEEP) {
        let source_id = match sa.source {
            Some(cid) if ctx.game.card(cid).zone == ZoneType::Battlefield => cid,
            _ => return,
        };
        let controller = ctx.game.card(source_id).controller;

        // 1. Add Age counter (mirrors Java host.addCounter(CounterEnumType.AGE, 1, ...))
        ctx.game
            .card_mut(source_id)
            .add_counter(&CounterType::Age, 1);

        // 2. Count age counters to determine how many times to pay
        let n = ctx
            .game
            .card(source_id)
            .counters
            .get(&CounterType::Age)
            .copied()
            .unwrap_or(0) as usize;

        // 3. Build merged cost: N copies of the base cost
        //    Mirrors Java Cost.mergeTo(cumCost, n, sa)
        let base_cost = crate::cost::parse_cost(&cum_cost_str);
        let mut merged_parts = Vec::new();
        for _ in 0..n {
            merged_parts.extend(base_cost.parts.clone());
        }
        let merged_cost = crate::cost::Cost {
            parts: merged_parts,
            has_tap: false,
            mandatory: false,
        };

        // 4. Pay the merged cost (payCostToPreventEffect flow)
        let paid = super::try_pay_cumulative_upkeep(ctx, sa, source_id, controller, &merged_cost);

        // 5. Fire PayCumulativeUpkeep trigger
        ctx.trigger_handler.run_trigger(
            TriggerType::PayCumulativeUpkeep,
            RunParams {
                card: Some(source_id),
                cumulative_upkeep_paid: Some(paid),
                ..Default::default()
            },
            false,
        );

        // 6. If not paid, sacrifice
        if !paid {
            do_sacrifice(ctx, sa, source_id, controller, None);
        }
        return;
    }

    let sac_valid = sa
        .params
        .get_cloned(keys::SAC_VALID)
        .unwrap_or_else(|| "Self".to_string());
    let defined = sa
        .params
        .get(keys::DEFINED)
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // How many permanents to sacrifice (e.g. Annihilator N).
    let amount: usize = sa
        .params
        .get(keys::AMOUNT)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Detect Exploit keyword sacrifice — fires TriggerType::Exploited after each sacrifice.
    let is_exploit = sa.params.is_true(keys::EXPLOIT);
    let exploit_source = if is_exploit { sa.source } else { None };

    let optional = sa.params.has(keys::OPTIONAL);
    let is_strict = sa.params.has(keys::STRICT_AMOUNT);

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
                Some(crate::ability::api_type::ApiType::Sacrifice),
            );
            if !accepted {
                continue;
            }
        }

        // When Optional$ True, Java uses choosePermanentsToSacrifice(min=0, max=amount)
        // which allows the player to sacrifice fewer than `amount` creatures.
        // We match this by collecting all chosen cards at once via choose_cards_for_effect.
        if optional
            && !sac_valid.eq_ignore_ascii_case("Self")
            && defined.strip_prefix("carduid_").is_none()
        {
            let valid: Vec<_> = ctx
                .game
                .cards_in_zone(ZoneType::Battlefield, sacrificing_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| matches_change_type(ctx.game.card(cid), &sac_valid, &[]))
                .filter(|&cid| {
                    !crate::staticability::static_ability_cant_sacrifice::cant_sacrifice(
                        &ctx.game.cards,
                        ctx.game.card(cid),
                        Some(sa),
                        false,
                    )
                })
                .collect();

            let min_targets = if is_strict { amount } else { 0 };
            let chosen = if valid.is_empty() {
                vec![]
            } else {
                ctx.agents[sacrificing_player.index()].choose_cards_for_effect(
                    sacrificing_player,
                    &valid,
                    min_targets,
                    amount,
                )
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
