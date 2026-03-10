use forge_foundation::{ColorSet, ZoneType};

use super::{matches_valid_cards, EffectContext};
use crate::card::AnimateState;
use crate::spellability::SpellAbility;

/// `SP$ AnimateAll` — animate all matching permanents on the battlefield.
///
/// Mirrors Java's `AnimateAllEffect.java`.
///
/// # Params
/// - `ValidCards` — filter for which cards to animate (e.g. "Land.YouCtrl", "Creature")
/// - `Power` — set base power
/// - `Toughness` — set base toughness
/// - `Types` — comma-separated types to add (e.g. "Creature,Elemental")
/// - `Keywords` — `&`-separated keywords to grant (until EOT)
/// - `Colors` — comma-separated colors to set (e.g. "Blue")
/// - `OverwriteColors` — if "True", replace color instead of adding
/// - `RemoveCreatureTypes` — if "True", clear subtypes before adding new types
/// - `RemoveAllAbilities` — if "True", clear all keywords/abilities
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let valid_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Card".to_string());

    let power_str = sa.params.get("Power").cloned();
    let toughness_str = sa.params.get("Toughness").cloned();
    let types_str = sa.params.get("Types").cloned();
    let keywords_str = sa.params.get("Keywords").cloned();
    let colors_str = sa.params.get("Colors").cloned();
    let overwrite_colors = sa
        .params
        .get("OverwriteColors")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);
    let remove_creature_types = sa
        .params
        .get("RemoveCreatureTypes")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);
    let remove_all_abilities = sa
        .params
        .get("RemoveAllAbilities")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    // Collect matching cards on the battlefield.
    let player_ids = ctx.game.player_order.clone();
    let mut targets = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_filter, controller) {
                targets.push(cid);
            }
        }
    }

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }

        // Save original state (only if not already animated this turn)
        if ctx.game.card(card_id).animate_state.is_none() {
            let card = &ctx.game.cards[card_id.index()];
            ctx.game.cards[card_id.index()].animate_state = Some(AnimateState {
                original_type_line: card.type_line.clone(),
                original_base_power: card.base_power,
                original_base_toughness: card.base_toughness,
                original_color: card.color,
            });
        }

        // RemoveAllAbilities — strip keywords/abilities
        if remove_all_abilities {
            ctx.game.cards[card_id.index()].keywords.clear();
            ctx.game.cards[card_id.index()].pump_keywords.clear();
            ctx.game.cards[card_id.index()].granted_keywords.clear();
        }

        // RemoveCreatureTypes — clear subtypes before adding new types
        if remove_creature_types {
            ctx.game.cards[card_id.index()].type_line.subtypes.clear();
        }

        // Apply type changes
        if let Some(ref types) = types_str {
            for t in types.split(',') {
                let t = t.trim();
                if !t.is_empty() {
                    ctx.game.cards[card_id.index()].type_line.add_type(t);
                }
            }
        }

        // Apply P/T
        if let Some(ref p) = power_str {
            if let Ok(val) = p.trim().parse::<i32>() {
                ctx.game.cards[card_id.index()].base_power = Some(val);
            }
        }
        if let Some(ref t) = toughness_str {
            if let Ok(val) = t.trim().parse::<i32>() {
                ctx.game.cards[card_id.index()].base_toughness = Some(val);
            }
        }

        // Apply keywords (until EOT — stored in pump_keywords so they get cleared at cleanup)
        // AnimateAll uses " & " as keyword separator (unlike Animate which uses ",")
        if let Some(ref kws) = keywords_str {
            for kw in kws.split('&') {
                let kw = kw.trim().to_string();
                if !kw.is_empty() {
                    ctx.game.cards[card_id.index()].pump_keywords.push(kw);
                }
            }
        }

        // Apply color
        if let Some(ref colors) = colors_str {
            let mut new_color = ColorSet::COLORLESS;
            for c in colors.split(',') {
                let c = c.trim().to_lowercase();
                match c.as_str() {
                    "white" | "w" => new_color = new_color.union(ColorSet::WHITE),
                    "blue" | "u" => new_color = new_color.union(ColorSet::BLUE),
                    "black" | "b" => new_color = new_color.union(ColorSet::BLACK),
                    "red" | "r" => new_color = new_color.union(ColorSet::RED),
                    "green" | "g" => new_color = new_color.union(ColorSet::GREEN),
                    _ => {}
                }
            }
            if overwrite_colors {
                ctx.game.cards[card_id.index()].color = new_color;
            } else {
                ctx.game.cards[card_id.index()].color =
                    ctx.game.cards[card_id.index()].color.union(new_color);
            }
        }

    }
}
