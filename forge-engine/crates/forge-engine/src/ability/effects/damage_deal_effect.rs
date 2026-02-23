use forge_foundation::ZoneType;

use super::{parse_param, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    println!("DEBUG: resolving damage_deal effect!");
    let damage = resolve_damage_amount(ctx, sa);
    println!("DEBUG: calculated damage amount: {}", damage);

    // For triggered abilities, resolve Defined$ for target
    let target_player = sa.target_chosen.target_player.or_else(|| {
        if let Some(defined) = sa.params.get("Defined") {
            resolve_defined_player(defined, sa.activating_player, ctx.game)
        } else {
            None
        }
    });

    if let Some(target_player) = target_player {
        println!("DEBUG: targeting player {:?}", target_player);
        ctx.game.deal_damage_to_player(target_player, damage);

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
    } else {
        println!("DEBUG: target_player is NONE");
    }
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            ctx.game.deal_damage_to_card(target_card, damage);

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

    if let Some(source_id) = sa.source {
        let svar_val = ctx.game.card(source_id).svars.get(var_name).cloned();
        if let Some(expr) = svar_val {
            return evaluate_svar_expr(ctx, &expr);
        }
    }

    0
}

/// Evaluate a simple SVar expression string.
/// Mirrors Java's `AbilityUtils.calculateAmount` for common SVar patterns.
fn evaluate_svar_expr(ctx: &EffectContext, expr: &str) -> i32 {
    match expr {
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
