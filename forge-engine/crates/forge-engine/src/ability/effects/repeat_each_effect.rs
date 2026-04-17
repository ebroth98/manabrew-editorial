use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_zone_type, resolve_defined_players, EffectContext};
use crate::parsing::keys;
use crate::spellability::{build_spell_ability, SpellAbility};

/// `SP$ RepeatEach` — loop a sub-ability over cards or players.
///
/// Mirrors Java's `RepeatEachEffect.java`.
///
/// # Params
/// - `RepeatSubAbility` — SVar name on source card for the sub-ability to resolve each iteration
/// - `RepeatCards` — if present, iterate over matching cards (filter string)
/// - `RepeatPlayers` — if present, iterate over matching players
/// - `Zone` — zone to search for RepeatCards (default Battlefield)
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let controller = sa.activating_player;
    let use_damage_map = sa.params.has("DamageMap");
    let use_change_zone_table = sa.params.has("ChangeZoneTable");

    if use_damage_map {
        ctx.game.ensure_pending_damage_maps();
    }
    if use_change_zone_table {
        ctx.game.ensure_pending_change_zone_table();
    }

    // Get the sub-ability SVar name
    let sub_svar_name = match sa.params.get_cloned(keys::REPEAT_SUB_ABILITY) {
        Some(name) => name,
        None => return,
    };

    // Look up the sub-ability text from the source card's SVars
    let sub_text = match ctx.game.card(source_id).svars.get(&sub_svar_name).cloned() {
        Some(text) => text,
        None => return,
    };

    // Determine iteration mode: cards or players
    if let Some(repeat_cards_filter) = sa.params.get_cloned(keys::REPEAT_CARDS) {
        // Card iteration path
        let zone = sa
            .params
            .get(keys::ZONE)
            .and_then(|s| parse_zone_type(s))
            .unwrap_or(ZoneType::Battlefield);

        // Collect matching cards
        let matching: Vec<crate::ids::CardId> = {
            let mut result = Vec::new();
            for &pid in &ctx.game.player_order.clone() {
                let zone_cards = ctx.game.cards_in_zone(zone, pid).to_vec();
                for cid in zone_cards {
                    if matches_valid_cards(ctx.game.card(cid), &repeat_cards_filter, controller) {
                        result.push(cid);
                    }
                }
            }
            result
        };

        // Iterate: remember card → resolve sub-SA → un-remember
        for card_id in matching {
            ctx.game.card_mut(source_id).clear_remembered();
            ctx.game.card_mut(source_id).add_remembered_card(card_id);

            // Build and resolve sub-ability
            let sub_sa = build_spell_ability(ctx.game, source_id, &sub_text, controller);
            resolve_sub_chain(ctx, sub_sa);

            if ctx.game.game_over {
                break;
            }
        }

        // Clean up remembered cards
        ctx.game.card_mut(source_id).clear_remembered();
    } else if let Some(repeat_players) = sa.params.get_cloned(keys::REPEAT_PLAYERS) {
        // Player iteration path
        let players = resolve_defined_players(&repeat_players, controller, ctx.game);

        for pid in players {
            ctx.game.card_mut(source_id).clear_remembered();
            ctx.game.card_mut(source_id).add_remembered_player(pid);

            let sub_sa = build_spell_ability(ctx.game, source_id, &sub_text, pid);
            resolve_sub_chain(ctx, sub_sa);

            if ctx.game.game_over {
                break;
            }
        }

        ctx.game.card_mut(source_id).clear_remembered();
    }

    if use_damage_map {
        // Mirror Java RepeatEach post-loop damage-map resolve.
        let mut flush_sa = sa.clone();
        flush_sa.damage_map = ctx.game.pending_damage_map.clone();
        flush_sa.prevent_map = ctx.game.pending_prevent_map.clone();
        super::damage_resolve_effect::resolve(ctx, &flush_sa);
        ctx.game.clear_pending_damage_maps();
    }
    if use_change_zone_table {
        if let Some(table) = ctx.game.pending_change_zone_table.clone() {
            table.trigger_changes_zone_all(ctx.trigger_handler, ctx.game, Some(sa));
            ctx.game.clear_pending_change_zone_table();
        }
    }
}

/// Walk a sub-ability chain (same pattern as charm_effect.rs).
fn resolve_sub_chain(ctx: &mut EffectContext, initial: SpellAbility) {
    let mut cur_opt: Option<SpellAbility> = Some(initial);
    while let Some(cur_sa) = cur_opt {
        super::resolve_effect(ctx, &cur_sa);
        cur_opt = cur_sa.sub_ability.map(|b| *b);
        if ctx.game.game_over {
            break;
        }
    }
}
