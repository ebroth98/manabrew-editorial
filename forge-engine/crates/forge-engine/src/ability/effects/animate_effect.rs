use forge_foundation::{ColorSet, ZoneType};

use super::EffectContext;
use crate::card::AnimateState;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::parse_trigger;

/// `SP$ Animate` — turn a non-creature permanent into a creature (or modify creature stats).
///
/// Mirrors Java's `AnimateEffectBase.java` + `AnimateEffect.java`.
///
/// # Params
/// - `Defined$` — target card(s) (default: source card itself)
/// - `Power` — set base power
/// - `Toughness` — set base toughness
/// - `Types` — comma-separated types to add (e.g. "Creature,Land")
/// - `Keywords` — comma-separated keywords to grant (until EOT)
/// - `Colors` — comma-separated colors to set (e.g. "White,Blue")
/// - `OverwriteTypes` — if "True", replace type_line instead of adding
///
/// The animate_state is saved so step_cleanup can restore the original card state.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Determine target card
    let target_ids = resolve_animate_targets(ctx, sa, controller);

    let power_str = sa.params.get_cloned(keys::POWER);
    let toughness_str = sa.params.get_cloned(keys::TOUGHNESS);
    let types_str = sa.params.get_cloned(keys::TYPES);
    let keywords_str = sa.params.get_cloned(keys::KEYWORDS);
    let colors_str = sa.params.get_cloned(keys::COLORS);
    let triggers_str = sa.params.get_cloned(keys::TRIGGERS);
    let overwrite_types = sa
        .params
        .get(keys::OVERWRITE_TYPES)
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    // Resolve Triggers$ SVars from the source card into parsed Trigger objects.
    // These will be temporarily added to each target card.
    let mut parsed_triggers: Vec<crate::trigger::Trigger> = Vec::new();
    if let Some(ref trigs) = triggers_str {
        // The source card holds the SVars that define the triggers.
        let source_id = sa.source.unwrap_or(crate::ids::CardId(0));
        let source_svars = ctx.game.card(source_id).svars.clone();
        let mut next_trig_id = 1000u32; // high base to avoid collisions
        for trig_name in trigs.split(',') {
            let trig_name = trig_name.trim();
            if trig_name.is_empty() {
                continue;
            }
            if let Some(svar_text) = source_svars.get(trig_name) {
                if let Some(mut trig) = parse_trigger(svar_text, &mut next_trig_id) {
                    trig.execute = trig
                        .execute
                        .clone();
                    parsed_triggers.push(trig);
                }
            }
        }
    }

    for card_id in target_ids {
        // Only animate cards on the battlefield
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

        // Apply type changes
        if let Some(ref types) = types_str {
            if overwrite_types {
                ctx.game.cards[card_id.index()].type_line = forge_foundation::CardTypeLine::new();
            }
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
        if let Some(ref kws) = keywords_str {
            for kw in kws.split(',') {
                let kw = kw.trim().to_string();
                if !kw.is_empty() {
                    ctx.game.cards[card_id.index()].pump_keywords.push(kw);
                }
            }
        }

        // Apply triggers (e.g. Supernatural Stamina's death → return trigger)
        // Triggers are added from the SOURCE card's SVars, not the target's.
        // Also copy the SVars needed by the trigger's Execute$ reference.
        if !parsed_triggers.is_empty() {
            let source_id = sa.source.unwrap_or(crate::ids::CardId(0));
            let source_svars = ctx.game.card(source_id).svars.clone();
            for trig in &parsed_triggers {
                ctx.game.cards[card_id.index()].triggers.push(trig.clone());
                ctx.game.cards[card_id.index()].pump_trigger_count += 1;
                // Copy the Execute SVar from source to target so trigger resolution
                // can find it (e.g. SupernaturalStaminaTrigChangeZone)
                if !trig.execute.is_empty() {
                    if let Some(exec_svar) = source_svars.get(&trig.execute) {
                        ctx.game.cards[card_id.index()]
                            .svars
                            .entry(trig.execute.clone())
                            .or_insert_with(|| exec_svar.clone());
                    }
                }
            }
            // Re-register this card's triggers so the new ones are active
            ctx.trigger_handler.register_active_trigger(ctx.game, card_id);
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
            ctx.game.cards[card_id.index()].color = new_color;
        }
    }
}

/// Resolve which card(s) to animate.
fn resolve_animate_targets(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    _controller: crate::ids::PlayerId,
) -> Vec<crate::ids::CardId> {
    // Check for explicit target
    if let Some(target) = sa.target_chosen.target_card {
        return vec![target];
    }

    // Check Defined$ param
    if let Some(defined) = sa.params.get(keys::DEFINED) {
        match defined {
            "Self" => {
                if let Some(src) = sa.source {
                    return vec![src];
                }
            }
            "ParentTarget" => {
                if let Some(pt) = ctx.parent_target_card {
                    return vec![pt];
                }
            }
            _ => {
                // Try as "Remembered" or other defined resolution
                if defined == "Remembered" {
                    if let Some(src) = sa.source {
                        return ctx.game.card(src).remembered_cards.clone();
                    }
                }
            }
        }
    }

    // Default: animate source card itself
    if let Some(src) = sa.source {
        vec![src]
    } else {
        vec![]
    }
}
