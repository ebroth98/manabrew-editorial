use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::card::CardInstance;
use crate::ids::CardId;
use crate::spellability::StackEntry;
use crate::trigger::parse_pipe_params;

pub fn resolve(
    ctx: &mut EffectContext,
    _params: &BTreeMap<String, String>,
    entry: &StackEntry,
    ability: &str,
) {
    // Clone a targeted permanent onto the battlefield under the controller's control.
    // Mirrors Java CopyPermanentEffect.
    // Supports: PumpKeywords$ (extra keywords on the copy).
    if let Some(original_id) = entry.target_card {
        if ctx.game.card(original_id).zone == ZoneType::Battlefield {
            let params = parse_pipe_params(ability);
            let original = ctx.game.card(original_id).clone();

            let mut copy = CardInstance::new(
                CardId(0),
                original.card_name.clone(),
                entry.controller,
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

            // Apply PumpKeywords$ (e.g. "Haste" added temporarily to the copy).
            if let Some(pump_kws) = params.get("PumpKeywords") {
                for kw in pump_kws.split(',') {
                    let kw = kw.trim().to_string();
                    if !kw.is_empty() {
                        copy.granted_keywords.push(kw);
                    }
                }
            }

            let copy_id = ctx.game.create_card(copy);
            ctx.game
                .move_card(copy_id, ZoneType::Battlefield, entry.controller);
            ctx.trigger_handler
                .register_active_trigger(ctx.game, copy_id);
            emit_zone_trigger(
                ctx.trigger_handler,
                copy_id,
                ZoneType::None,
                ZoneType::Battlefield,
            );
        }
    }
}
