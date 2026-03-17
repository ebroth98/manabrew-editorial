use forge_foundation::ZoneType;

use super::{parse_param, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let damage = resolve_damage_amount(ctx, sa);

    // For triggered abilities, resolve Defined$ for target
    let target_player = sa.target_chosen.target_player.or_else(|| {
        if let Some(defined) = sa.defined() {
            resolve_defined_player(defined, sa.activating_player, ctx.game)
        } else {
            None
        }
    });

    // Check source card for Infect/Wither keywords
    let (source_has_infect_keyword, source_has_wither) = if let Some(src_id) = sa.source {
        let src = ctx.game.card(src_id);
        (
            src.has_infect(),
            src.has_wither()
                || crate::staticability::static_ability_wither_damage::is_wither_damage(
                    &ctx.game.cards,
                    src,
                ),
        )
    } else {
        (false, false)
    };

    // Overload: deal damage to ALL valid creatures instead of the chosen target.
    if sa.overloaded {
        let valid_tgts = sa.params.get("ValidTgts").cloned().unwrap_or_default();
        let all_bf: Vec<crate::ids::CardId> = ctx
            .game
            .player_order
            .clone()
            .iter()
            .flat_map(|&pid| ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec())
            .collect();
        for cid in all_bf {
            if ctx.game.card(cid).zone != ZoneType::Battlefield {
                continue;
            }
            if !super::matches_valid_cards(ctx.game.card(cid), &valid_tgts, sa.activating_player) {
                continue;
            }
            // Track damage source for DamagedBy trigger filters
            if let Some(src_id) = sa.source {
                if !ctx
                    .game
                    .card(cid)
                    .damage_sources_this_turn
                    .contains(&src_id)
                {
                    ctx.game.card_mut(cid).damage_sources_this_turn.push(src_id);
                }
            }
            if source_has_infect_keyword || source_has_wither {
                if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                    &ctx.game.cards,
                    ctx.game.card(cid),
                    &crate::card::CounterType::M1M1,
                ) {
                    ctx.game
                        .card_mut(cid)
                        .add_counter(&crate::card::CounterType::M1M1, damage);
                }
            } else {
                ctx.game.deal_damage_to_card(cid, damage);
            }
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::DamageDone,
                crate::event::RunParams {
                    damage_source: sa.source,
                    damage_target_card: Some(cid),
                    damage_amount: Some(damage),
                    is_combat_damage: Some(false),
                    ..Default::default()
                },
                false,
            );
        }
        return;
    }

    if let Some(target_player) = target_player {
        let source_has_infect = if let Some(src_id) = sa.source {
            let src = ctx.game.card(src_id);
            source_has_infect_keyword
                || crate::staticability::static_ability_infect_damage::is_infect_damage(
                    ctx.game,
                    &ctx.game.cards,
                    target_player,
                    src.controller,
                )
        } else {
            false
        };
        if source_has_infect {
            // Infect: deal damage to players as poison counters
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &ctx.game.cards,
                target_player,
                &crate::card::CounterType::Poison,
            ) {
                ctx.game.player_mut(target_player).poison_counters += damage;
            }
        } else {
            ctx.game.deal_damage_to_player(target_player, damage);
        }

        // Fire DamageDone trigger
        ctx.trigger_handler.run_trigger(
            crate::event::TriggerType::DamageDone,
            crate::event::RunParams {
                damage_source: sa.source,
                damage_target_player: Some(target_player),
                damage_amount: Some(damage),
                is_combat_damage: Some(false),
                ..Default::default()
            },
            false,
        );
    }
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            // Protection: prevents all damage from matching sources
            if let Some(src_id) = sa.source {
                if crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
                    &ctx.game.cards,
                    ctx.game.card(target_card),
                    ctx.game.card(src_id),
                ) {
                    return;
                }
            }

            // Track damage source for DamagedBy trigger filters
            if let Some(src_id) = sa.source {
                if !ctx
                    .game
                    .card(target_card)
                    .damage_sources_this_turn
                    .contains(&src_id)
                {
                    ctx.game
                        .card_mut(target_card)
                        .damage_sources_this_turn
                        .push(src_id);
                }
            }
            if source_has_infect_keyword || source_has_wither {
                // Infect/Wither: damage to creatures as -1/-1 counters
                if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                    &ctx.game.cards,
                    ctx.game.card(target_card),
                    &crate::card::CounterType::M1M1,
                ) {
                    ctx.game
                        .card_mut(target_card)
                        .add_counter(&crate::card::CounterType::M1M1, damage);
                }
            } else {
                ctx.game.deal_damage_to_card(target_card, damage);
            }

            // Fire DamageDone trigger
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::DamageDone,
                crate::event::RunParams {
                    damage_source: sa.source,
                    damage_target_card: Some(target_card),
                    damage_amount: Some(damage),
                    is_combat_damage: Some(false),
                    ..Default::default()
                },
                false,
            );

            if sa
                .params
                .get("RememberDamagedCreature")
                .is_some_and(|v| v.eq_ignore_ascii_case("True"))
            {
                if let Some(src_id) = sa.source {
                    let src = ctx.game.card_mut(src_id);
                    if !src.remembered_cards.contains(&target_card) {
                        src.remembered_cards.push(target_card);
                    }
                }
            }
        }
    }
}

/// Resolve the NumDmg$ parameter, supporting both integer literals and SVar
/// references (e.g. `NumDmg$ X` where `SVar:X:ParentTargeted$CardPower`).
/// Mirrors Java's `AbilityUtils.calculateAmount(sa, "NumDmg", sa)`.
fn resolve_damage_amount(ctx: &EffectContext, sa: &SpellAbility) -> i32 {
    // Fast path: NumDmg$ is a direct integer
    if let Some(n) = parse_param(&sa.ability_text, "NumDmg$ ") {
        return n;
    }

    // NumDmg$ <var> — look up the SVar on the source card and evaluate it.
    // Example: NumDmg$ X with SVar:X:ParentTargeted$CardPower
    let var_name = match sa.params.get("NumDmg") {
        Some(v) if !v.is_empty() => v.as_str(),
        _ => return 0,
    };

    // Check if var_name is "X" — use x_mana_cost_paid
    if var_name == "X" {
        if let Some(source_id) = sa.source {
            if let Some(svar_expr) = ctx.game.card(source_id).svars.get("X") {
                return evaluate_svar_expr(ctx, sa, svar_expr);
            }
        }
        return sa.x_mana_cost_paid as i32;
    }

    if let Some(source_id) = sa.source {
        let svar_val = ctx.game.card(source_id).svars.get(var_name).cloned();
        if let Some(expr) = svar_val {
            return evaluate_svar_expr(ctx, sa, &expr);
        }
    }

    0
}

/// Evaluate a simple SVar expression string.
/// Mirrors Java's `AbilityUtils.calculateAmount` for common SVar patterns.
fn evaluate_svar_expr(ctx: &EffectContext, sa: &SpellAbility, expr: &str) -> i32 {
    // Count$Kicked.X.Y — delegate to shared evaluator
    if expr.starts_with("Count$Kicked.") {
        return super::evaluate_svar(expr, sa);
    }
    match expr {
        // X mana cost paid value
        "Count$xPaid" | "Count$XPaid" => sa.x_mana_cost_paid as i32,
        // Power / toughness of the parent SA's chosen target card.
        // Used by Ram Through: SVar:X:ParentTargeted$CardPower
        "ParentTargeted$CardPower" => ctx
            .parent_target_card
            .map(|id| ctx.game.card(id).power())
            .unwrap_or(0),
        "ParentTargeted$CardToughness" => ctx
            .parent_target_card
            .map(|id| ctx.game.card(id).toughness())
            .unwrap_or(0),
        _ => 0,
    }
}
