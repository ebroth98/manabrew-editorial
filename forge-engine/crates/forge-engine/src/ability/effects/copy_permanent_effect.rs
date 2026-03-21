use forge_foundation::color::ColorSet;
use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::card::CardInstance;
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

    let mut copy = CardInstance::new(
        CardId(0),
        original.card_name.clone(),
        sa.activating_player,
        original.type_line.clone(),
        original.mana_cost.clone(),
        original.color,
        original.base_power,
        original.base_toughness,
        original.keywords.clone(),
        original.abilities.clone(),
    );
    copy.triggers = original.triggers.clone();
    copy.svars = original.svars.clone();
    copy.static_abilities = original.static_abilities.clone();
    copy.replacement_effects = original.replacement_effects.clone();
    // Copies are tokens for zone-change purposes (cease to exist off battlefield).
    copy.is_token = true;

    // Apply SetColor$ (e.g. Embalm sets color to White).
    if let Some(set_color) = sa.params.get(keys::SET_COLOR) {
        copy.color = ColorSet::from_names(set_color);
    }

    // Apply AddTypes$ (e.g. Embalm adds "Zombie").
    if let Some(add_types) = sa.params.get(keys::ADD_TYPES) {
        for t in add_types.split(" & ") {
            let t = t.trim();
            if !t.is_empty() && !copy.type_line.subtypes.contains(&t.to_string()) {
                copy.type_line.subtypes.push(t.to_string());
            }
        }
    }

    // Apply SetPower$/SetToughness$ (e.g. Eternalize sets to 4/4).
    if let Some(p) = sa
        .params
        .get(keys::SET_POWER)
        .and_then(|v| v.parse::<i32>().ok())
    {
        copy.base_power = Some(p);
    }
    if let Some(t) = sa
        .params
        .get(keys::SET_TOUGHNESS)
        .and_then(|v| v.parse::<i32>().ok())
    {
        copy.base_toughness = Some(t);
    }

    // Apply PumpKeywords$ (e.g. "Haste" added temporarily to the copy).
    if let Some(pump_kws) = sa.params.get(keys::PUMP_KEYWORDS) {
        for kw in pump_kws.split(',') {
            let kw = kw.trim().to_string();
            if !kw.is_empty() {
                copy.keywords.push(kw);
            }
        }
    }

    // Apply AddKeywords$ (e.g. additional keywords on the copy).
    if let Some(add_kws) = sa.params.get(keys::ADD_KEYWORDS) {
        for kw in add_kws.split(" & ") {
            let kw = kw.trim().to_string();
            if !kw.is_empty() {
                copy.keywords.push(kw);
            }
        }
    }

    // Strip mana cost for Embalm/Eternalize copies (they have no mana cost).
    if sa
        .params
        .get(keys::SET_MANA_COST)
        .map_or(false, |v| v == "0" || v.is_empty())
    {
        copy.mana_cost = forge_foundation::mana::ManaCost::no_cost();
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
}

fn resolve_original(sa: &SpellAbility) -> Option<CardId> {
    // Check Defined$ parameter first.
    if let Some(defined) = sa.params.get(keys::DEFINED) {
        match defined {
            "Self" => return sa.source,
            "ParentTarget" => return sa.target_chosen.target_card,
            _ => {}
        }
    }
    // Fall back to targeting.
    sa.target_chosen.target_card
}
