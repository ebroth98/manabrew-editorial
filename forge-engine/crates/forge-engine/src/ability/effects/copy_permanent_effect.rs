use forge_foundation::color::ColorSet;
use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::card::Card;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `CopyPermanentEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(CopyPermanentEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    // Clone a permanent onto the battlefield under the controller's control.
    // Mirrors Java CopyPermanentEffect.
    // Supports: Defined$, SetColor$, AddTypes$, PumpKeywords$.

    eprintln!(
        "[copyperm-debug] T{} {:?} resolve source={:?} defined={:?} trigger_remembered_len={}",
        ctx.game.turn.turn_number, ctx.game.turn.phase, sa.source,
        sa.params.get(keys::DEFINED), sa.trigger_remembered.len()
    );

    // Resolve the card to copy: Defined$ first, then targeting.
    let original_id = resolve_original(sa);
    let Some(original_id) = original_id else {
        eprintln!("[copyperm-debug]   resolve_original returned None - BAILING");
        return;
    };
    eprintln!("[copyperm-debug]   original_id={:?} name={}", original_id, ctx.game.card(original_id).card_name);

    // For targeted copies, require the original on the battlefield.
    // For Defined$ Self (Embalm/Eternalize), the card may be in any zone.
    let is_defined = sa.params.has(keys::DEFINED);
    if !is_defined && ctx.game.card(original_id).zone != ZoneType::Battlefield {
        return;
    }

    let original = ctx.game.card(original_id).clone();
    let copy = get_proto_type(sa, &original, sa.activating_player);
    let copy_id = ctx.game.create_card(copy);
    eprintln!("[copyperm-debug]   created copy_id={:?} name={} triggers_count={}",
        copy_id, ctx.game.card(copy_id).card_name, ctx.game.card(copy_id).triggers.len());
    rebind_copied_traits(ctx.game, copy_id);
    ctx.game
        .move_card(copy_id, ZoneType::Battlefield, sa.activating_player);
    ctx.trigger_handler
        .register_active_trigger(ctx.game, copy_id);
    eprintln!("[copyperm-debug]   moved to battlefield, emitting ETB trigger");
    emit_zone_trigger(
        ctx.trigger_handler,
        copy_id,
        ZoneType::None,
        ZoneType::Battlefield,
    );
    ctx.trigger_handler.flush_waiting_triggers(ctx.game);
    eprintln!("[copyperm-debug]   flushed waiting triggers");

    // `AtEOT$ <action>` — register an end-of-turn delayed trigger targeting
    // the created copy (Java CopyPermanentEffect → registerDelayedTrigger).
    if let Some(action) = sa.params.get(keys::AT_EOT) {
        crate::ability::spell_ability_effect::register_at_eot(
            ctx.trigger_handler,
            ctx.game,
            sa,
            action,
            vec![copy_id],
        );
    }

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

/// Build the in-memory copy of `original` that a Copy/Embalm/Eternalize effect
/// will place onto the battlefield. Mirrors Java
/// `CopyPermanentEffect.getProtoType(SpellAbility, Card, Player)`.
///
/// Returned `Card` carries a placeholder `CardId(0)`; callers must invoke
/// `GameState::create_card` to receive the real id. Mana-cost strip,
/// `SetColor`, `AddTypes`, `SetPower/Toughness`, `PumpKeywords`, `AddKeywords`
/// are all applied here. Transformable / back-side handling and
/// `DefinedName` image re-keying are Java-only today (Rust doesn't have a
/// paper-card backing store to rebind); see the Java impl at L275 for the
/// full prototype pipeline.
pub fn get_proto_type(sa: &SpellAbility, original: &Card, new_owner: crate::ids::PlayerId) -> Card {
    let mut copy = Card::new(
        CardId(0),
        original.card_name.clone(),
        new_owner,
        original.type_line.clone(),
        original.mana_cost.clone(),
        original.color,
        original.base_power,
        original.base_toughness,
        original.keywords.as_string_list(),
        original.abilities.clone(),
    );
    copy.set_triggers(original.copiable_triggers());
    copy.set_svars_map(original.svars.clone());
    copy.set_static_abilities(original.static_abilities.clone());
    copy.set_replacement_effects(original.copiable_replacement_effects());
    copy.set_perpetual(original, false);
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

    copy
}

fn rebind_copied_traits(game: &mut crate::game::GameState, copy_id: CardId) {
    {
        // Rebuild activated/spell abilities after create_card assigns the real
        // CardId so copied abilities don't keep the placeholder id from
        // Card::new(CardId(0), ...).
        let copy = game.card_mut(copy_id);
        copy.update_spell_abilities();
    }

    let bound_host = game.card(copy_id).clone();
    let copy = game.card_mut(copy_id);

    for trigger in &mut copy.triggers {
        trigger.bind_host_card(bound_host.clone());
    }
    for static_ability in &mut copy.static_abilities {
        static_ability.base.set_host_card(bound_host.clone());
    }
    for replacement_effect in &mut copy.replacement_effects {
        replacement_effect.base.set_host_card(bound_host.clone());
    }
}

fn resolve_original(sa: &SpellAbility) -> Option<CardId> {
    // Check Defined$ parameter first.
    if let Some(defined) = sa.defined() {
        match defined {
            "Self" => return sa.source,
            "ParentTarget" => return sa.target_chosen.target_card,
            "TriggeredCard" | "TriggeredCardLKICopy" | "TriggeredSacrificedCard" => {
                // The triggering object for a Sacrificed/ChangesZone trigger
                // is stored under the "Card" key; read it back as a CardId.
                if let Some(card) = sa.get_triggering_card(crate::ability::AbilityKey::Card) {
                    return Some(card);
                }
                return None;
            }
            _ => {}
        }
    }
    // Fall back to targeting.
    sa.target_chosen.target_card
}
