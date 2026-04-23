use crate::ability::effects::{evaluate_svar, resolve_defined_player, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::trigger::parse_trigger;

/// Mirrors Java's `DelayedTriggerEffect`.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `DelayedTriggerEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(DelayedTriggerEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let Some(source_id) = sa.source else {
        return;
    };

    let mut next_id = 0;
    let Some(parsed) = parse_trigger(&sa.ability_text, &mut next_id) else {
        return;
    };
    let mode = parsed.kind;

    let execute_svar = if let Some(exec) = sa.params.get(keys::EXECUTE) {
        if let Some(svar_text) = ctx.game.card(source_id).get_s_var(exec) {
            svar_text.to_string()
        } else {
            exec.to_string()
        }
    } else {
        return;
    };

    let mut remembered_amount = 0;
    if sa
        .params
        .get(keys::REMEMBER_NUMBER)
        .is_some_and(|v| v.eq_ignore_ascii_case("True"))
    {
        remembered_amount += ctx
            .game
            .card(source_id)
            .remembered_cmc
            .iter()
            .copied()
            .sum::<i32>();
    }
    if let Some(svar_name) = sa.params.get(keys::REMEMBER_SVAR_AMOUNT) {
        if let Some(expr) = ctx.game.card(source_id).get_s_var(svar_name) {
            remembered_amount += evaluate_svar(expr, sa);
        }
    }

    let controller = if let Some(def_player) = sa.params.get(keys::DELAYED_TRIGGER_DEFINED_PLAYER) {
        resolve_defined_player(def_player, sa.activating_player, ctx.game)
            .unwrap_or(sa.activating_player)
    } else {
        sa.activating_player
    };
    let mut remembered_lki_cards = Vec::new();
    if sa.params.get(keys::REMEMBER_OBJECTS).is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim() == "RememberedLKI")
    }) {
        remembered_lki_cards = ctx.game.card(source_id).remembered_cards.clone();
    }
    // `RememberObjects$ Remembered` — snapshot the source card's current
    // remembered_cards into the delayed trigger so the executed ability sees
    // them later via `SpellAbility::trigger_remembered`. Ashling uses this to
    // track the token copy it created for its end-step sacrifice clause.
    let mut remembered_cards: Vec<crate::ids::CardId> =
        match sa.params.get(keys::REMEMBER_OBJECTS) {
            Some("Remembered") => ctx.game.card(source_id).remembered_cards.clone(),
            _ => Vec::new(),
        };

    // `RememberObjects$ TriggeredAttackerLKICopy` — snapshot the attacker
    // that fired the parent trigger so the delayed trigger's effect can
    // phase it out / operate on it at a later phase. Teferi's Veil uses
    // this to remember each attacker and phase them out at end of combat.
    // The attacker id is populated into both `remembered_cards` (so
    // `Defined$ DelayTriggerRememberedLKI` resolves via `trigger_remembered`)
    // and `remembered_lki_cards` (for the trigger_objects string lookup).
    if sa.params.get(keys::REMEMBER_OBJECTS).is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim() == "TriggeredAttackerLKICopy")
    }) {
        if let Some(attacker_str) =
            sa.get_triggering_object(crate::ability::AbilityKey::Attacker)
        {
            if let Ok(id) = attacker_str.parse::<u32>() {
                let cid = crate::ids::CardId(id);
                remembered_lki_cards.push(cid);
                if !remembered_cards.contains(&cid) {
                    remembered_cards.push(cid);
                }
            }
        }
    }

    let delayed = crate::trigger::handler::DelayedTrigger {
        mode,
        trigger_mode: parsed.mode,
        params: parsed.params,
        execute_svar,
        controller,
        source_card: source_id,
        created_turn: ctx.game.turn.turn_number,
        created_phase: ctx.game.turn.phase,
        target_card: None,
        remembered_amount,
        remembered_cards,
        remembered_lki_cards,
        sort_after_active: false,
    };
    if sa.params.has("ThisTurn") {
        ctx.trigger_handler
            .register_this_turn_delayed_trigger(delayed);
    } else if sa.params.has(keys::DELAYED_TRIGGER_DEFINED_PLAYER) {
        ctx.trigger_handler
            .register_player_defined_delayed_trigger(controller, delayed);
    } else {
        ctx.trigger_handler.register_delayed_trigger(delayed);
    }
}

/// End-of-turn / next-turn registration callback.
pub fn run(ctx: &mut EffectContext, sa: &SpellAbility) {
    use crate::ability::spell_ability_effect::SpellAbilityEffect;
    DelayedTriggerEffect::resolve(ctx, sa);
}
