use forge_foundation::color::ColorSet;
use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::card::Card;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Clone a permanent onto the battlefield under the controller's control.
    // Mirrors Java CopyPermanentEffect.
    // Supports: Defined$, SetColor$, AddTypes$, PumpKeywords$.

    // Resolve the card to copy: Defined$ first, then targeting.
    let original_id = resolve_original(sa);
    let Some(original_id) = original_id else {
        return;
    };

    // For targeted copies, require the original on the battlefield.
    // For Defined$ Self (Embalm/Eternalize), the card may be in any zone.
    let is_defined = sa.params.has(keys::DEFINED);
    if !is_defined && ctx.game.card(original_id).zone != ZoneType::Battlefield {
        return;
    }

    let original = ctx.game.card(original_id).clone();

    let mut copy = Card::new(
        CardId(0),
        original.card_name.clone(),
        sa.activating_player,
        original.type_line.clone(),
        original.mana_cost.clone(),
        original.color,
        original.base_power,
        original.base_toughness,
        original.keywords.as_string_list(),
        original.abilities.clone(),
    );
    copy.set_triggers(original.triggers.clone());
    copy.set_svars_map(original.svars.clone());
    copy.set_static_abilities(original.static_abilities.clone());
    copy.set_replacement_effects(original.replacement_effects.clone());
    copy.set_perpetual(&original, false);
    // Copies are tokens for zone-change purposes (cease to exist off battlefield).
    copy.set_is_token(true);

    // Apply SetColor$ (e.g. Embalm sets color to White).
    if let Some(set_color) = sa.params.get(keys::SET_COLOR) {
        copy.set_color(ColorSet::from_names(set_color));
    }

    // Apply AddTypes$ (e.g. Embalm adds "Zombie").
    if let Some(add_types) = sa.params.get(keys::ADD_TYPES) {
        for t in add_types.split(" & ") {
            let t = t.trim();
            if !t.is_empty() {
                copy.add_type(t);
            }
        }
    }

    // Apply SetPower$/SetToughness$ (e.g. Eternalize sets to 4/4).
    if let Some(p) = sa
        .params
        .get(keys::SET_POWER)
        .and_then(|v| v.parse::<i32>().ok())
    {
        copy.set_base_power(Some(p));
    }
    if let Some(t) = sa
        .params
        .get(keys::SET_TOUGHNESS)
        .and_then(|v| v.parse::<i32>().ok())
    {
        copy.set_base_toughness(Some(t));
    }

    // Apply PumpKeywords$ (e.g. "Haste" added temporarily to the copy).
    if let Some(pump_kws) = sa.params.get(keys::PUMP_KEYWORDS) {
        for kw in pump_kws.split(',') {
            let kw = kw.trim();
            if !kw.is_empty() {
                copy.add_intrinsic_keyword(kw);
            }
        }
    }

    // Apply AddKeywords$ (e.g. additional keywords on the copy).
    if let Some(add_kws) = sa.params.get(keys::ADD_KEYWORDS) {
        for kw in add_kws.split(" & ") {
            let kw = kw.trim();
            if !kw.is_empty() {
                copy.add_intrinsic_keyword(kw);
            }
        }
    }

    // Strip mana cost for Embalm/Eternalize copies (they have no mana cost).
    if sa
        .params
        .get(keys::SET_MANA_COST)
        .map_or(false, |v| v == "0" || v.is_empty())
    {
        copy.set_mana_cost(forge_foundation::mana::ManaCost::no_cost());
    }

    let copy_id = ctx.game.create_card(copy);
    ctx.game
        .move_card(copy_id, ZoneType::Battlefield, sa.activating_player);
    ctx.trigger_handler
        .register_active_trigger(ctx.game, copy_id);
    emit_zone_trigger(
        ctx.trigger_handler,
        copy_id,
        ZoneType::None,
        ZoneType::Battlefield,
    );

    // `RememberTokens$ True` — track each created token on the source card so
    // a downstream SubAbility (e.g. Ashling's `DelTrig` with
    // `RememberObjects$ Remembered`) can find it later.
    let remember_tokens = sa
        .params
        .get("RememberTokens")
        .map_or(false, |v| v.eq_ignore_ascii_case("True"));
    if remember_tokens {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).add_remembered_card(copy_id);
        }
    }
}

fn resolve_original(sa: &SpellAbility) -> Option<CardId> {
    // Check Defined$ parameter first.
    if let Some(defined) = sa.params.get(keys::DEFINED) {
        match defined {
            "Self" => return sa.source,
            "ParentTarget" => return sa.target_chosen.target_card,
            "TriggeredCard" | "TriggeredCardLKICopy" | "TriggeredSacrificedCard" => {
                // The triggering object for a Sacrificed/ChangesZone trigger
                // is stored under the "Card" key; read it back as a CardId.
                if let Some(id_str) = sa.trigger_objects.get("Card") {
                    if let Ok(id) = id_str.parse::<u32>() {
                        return Some(CardId(id));
                    }
                }
                return None;
            }
            _ => {}
        }
    }
    // Fall back to targeting.
    sa.target_chosen.target_card
}
