use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_param, resolve_numeric_svar, EffectContext};
use crate::card::card_damage_map::DamageTarget;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ EachDamage` — each matching creature/player deals damage.
///
/// Mirrors Java's `DamageEachEffect.java`.
/// - `ValidCards$` — which creatures deal damage.
/// - `NumDmg$` — how much damage each deals (default: power of the creature).
/// - `DefinedPlayers$` — if set, damage is dealt to matching players.
///
/// # Card script examples
/// ```text
/// A:SP$ EachDamage | ValidCards$ Creature.YouCtrl | NumDmg$ X
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let use_damage_map = ctx.game.pending_damage_map.is_some() || sa.params.has("DamageMap");
    if sa.params.has("DamageMap") {
        ctx.game.ensure_pending_damage_maps();
    }

    let valid_filter = sa
        .params
        .get("ValidCards")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Creature".to_string());
    let fixed_dmg = parse_param(&sa.ability_text, "NumDmg$ ").or_else(|| {
        let v = resolve_numeric_svar(ctx.game, sa, "NumDmg", -1);
        if v == -1 {
            None
        } else {
            Some(v)
        }
    });

    let player_ids = ctx.game.player_order.clone();
    let mut damagers: Vec<CardId> = Vec::new();

    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_filter, sa.activating_player) {
                damagers.push(cid);
            }
        }
    }

    // Determine damage target: opponent by default
    let target_player = sa
        .target_chosen
        .target_player
        .unwrap_or_else(|| ctx.game.opponent_of(sa.activating_player));

    for card_id in damagers {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }
        let dmg = fixed_dmg.unwrap_or_else(|| ctx.game.card(card_id).power().max(0));
        if dmg <= 0 {
            continue;
        }

        if use_damage_map {
            if let Some(map) = ctx.game.pending_damage_map.as_mut() {
                map.put(card_id, DamageTarget::Player(target_player), dmg);
            }
        } else {
            // Deal damage to the target player
            let dealt = ctx.game.deal_damage_to_player(target_player, dmg);
            ctx.game.record_player_damage_assignment(
                Some(card_id),
                Some(target_player),
                dealt,
                false,
            );

            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::DamageDone,
                crate::event::RunParams {
                    damage_source: Some(card_id),
                    damage_target_player: Some(target_player),
                    damage_amount: Some(dmg),
                    is_combat_damage: Some(false),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
