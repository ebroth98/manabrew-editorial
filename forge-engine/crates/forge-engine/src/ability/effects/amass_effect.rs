//! Amass effect — create or grow an Army token.
//!
//! Ported from Java's `AmassEffect.java`.
//!
//! Amass N {Type}: Put N +1/+1 counters on an Army you control.
//! If you don't control one, create a 0/0 black {Type} Army creature token first.
//! If the Army isn't a {Type}, it becomes a {Type} in addition to its other types.

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use crate::parsing::keys;

use super::{emit_zone_trigger, parse_counter_type, EffectContext};
use crate::card::Card;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::spellability::SpellAbility;
use crate::staticability::parse_static_ability;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);
    let amass_type = sa
        .params
        .get("Type")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Zombie".to_string());

    // Step 1: If no Army on battlefield, create one
    let has_army = ctx
        .game
        .cards
        .iter()
        .any(|c| {
            c.zone == ZoneType::Battlefield
                && c.controller == controller
                && c.type_line.subtypes.iter().any(|s| s.eq_ignore_ascii_case("Army"))
        });

    if !has_army {
        create_army_token(ctx, sa, controller, &amass_type);
    }

    // Step 2: Find all Armies
    let armies: Vec<CardId> = ctx
        .game
        .cards
        .iter()
        .filter(|c| {
            c.zone == ZoneType::Battlefield
                && c.controller == controller
                && c.type_line.subtypes.iter().any(|s| s.eq_ignore_ascii_case("Army"))
        })
        .map(|c| c.id)
        .collect();

    if armies.is_empty() {
        return;
    }

    // Step 3: Choose an Army (auto-select if only one)
    let target = if armies.len() == 1 {
        armies[0]
    } else {
        ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[controller.index()]
            .choose_single_card_for_zone_change(
                controller,
                &armies,
                "Choose an Army",
                false,
            )
            .unwrap_or(armies[0])
    };

    // RememberAmass$
    if sa.param_is_true(keys::REMEMBER_AMASS) {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).add_remembered_card(target);
        }
    }

    // Step 4: Add +1/+1 counters
    let counter_type = parse_counter_type("P1P1");
    ctx.game.card_mut(target).add_counter(&counter_type, amount);

    // Step 5: If Army doesn't have the amass type, add it via effect
    let has_type = ctx
        .game
        .card(target)
        .type_line
        .subtypes
        .iter()
        .any(|s| s.eq_ignore_ascii_case(&amass_type));

    if !has_type {
        add_type_effect(ctx, sa, controller, target, &amass_type);
    }
}

/// Create a 0/0 black {Type} Army creature token.
fn create_army_token(
    ctx: &mut EffectContext,
    _sa: &SpellAbility,
    controller: crate::ids::PlayerId,
    amass_type: &str,
) {
    let token_name = format!("{} Army Token", amass_type);
    let type_str = format!("Creature - {} Army", amass_type);

    let mut token = Card::new(
        CardId(0),
        token_name,
        controller,
        CardTypeLine::parse(&type_str),
        ManaCost::parse(""),
        ColorSet::BLACK,
        Some(0),
        Some(0),
        vec![],
        vec![],
    );
    token.set_controller(controller);
    token.set_is_token(true);

    // RNG sync: match Java's token art selection
    ctx.rng.next_int(1);
    ctx.rng.next_int(1);

    let token_id = ctx.game.create_card(token);
    ctx.game.move_card(token_id, ZoneType::Battlefield, controller);

    ctx.trigger_handler
        .register_active_trigger(ctx.game, token_id);

    ctx.trigger_handler.run_trigger(
        TriggerType::TokenCreated,
        RunParams {
            card: Some(token_id),
            player: Some(controller),
            ..Default::default()
        },
        false,
    );

    emit_zone_trigger(
        ctx.trigger_handler,
        token_id,
        ZoneType::None,
        ZoneType::Battlefield,
    );
}

/// Add a creature type to the Army via a command-zone continuous effect.
/// Mirrors Java's createEffect + AddType$ static ability (lines 101-110).
fn add_type_effect(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    controller: crate::ids::PlayerId,
    target: CardId,
    amass_type: &str,
) {
    let mut effect = Card::new(
        CardId(0),
        "Amass Effect".to_string(),
        controller,
        CardTypeLine::parse("Effect"),
        ManaCost::parse("0"),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    );
    effect.set_controller(controller);
    effect.set_effect_source(sa.source);
    effect.add_remembered_card(target);
    effect.set_temp_effect_host(Some(target)); // Removed when target leaves play

    let static_text = format!(
        "Mode$ Continuous | Affected$ Card.IsRemembered | EffectZone$ Command | AddType$ {}",
        amass_type
    );
    if let Some(parsed) = parse_static_ability(&format!("S$ {}", static_text)) {
        effect.add_static_ability(parsed);
    }

    let effect_id = ctx.game.create_card(effect);
    ctx.game
        .move_card(effect_id, ZoneType::Command, controller);
}
