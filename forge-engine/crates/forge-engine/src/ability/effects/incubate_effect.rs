//! Incubate effect — create Incubator artifact token with +1/+1 counters.
//!
//! Ported 1:1 from Java's `IncubateEffect.java`.
//! Incubate N: Create an Incubator token with N +1/+1 counters on it.
//! (It's a transforming double-faced token. Pay {2}: Transform it.
//! It becomes a 0/0 Phyrexian artifact creature.)

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use super::{emit_zone_trigger, parse_counter_type, EffectContext};
use crate::card::Card;
use crate::event::RunParams;
use crate::ids::CardId;
use crate::spellability::SpellAbility;
use crate::trigger::TriggerType;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `IncubateEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(IncubateEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(0);
    let times = super::resolve_numeric_svar(ctx.game, sa, "Times", 1).max(0) as usize;
    let controller = sa.activating_player;

    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    for &pid in &players {
        for _ in 0..times {
            create_incubator_token(ctx, sa, pid, amount);
        }
    }
}

/// Create a single Incubator token with N +1/+1 counters.
fn create_incubator_token(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    player: crate::ids::PlayerId,
    counter_amount: i32,
) {
    // Try registered token template first
    if let Some(template) = ctx
        .token_templates
        .get("incubator_c_0_0_a_phyrexian")
        .cloned()
    {
        ctx.sync_token_art_rng("incubator_c_0_0_a_phyrexian", sa);

        let mut token = template;
        token.set_owner(player);
        token.set_controller(player);
        token.set_is_token(true);

        let token_id = ctx.game.create_card(token);
        ctx.move_card(token_id, ZoneType::Battlefield, player);

        // Add +1/+1 counters
        if counter_amount > 0 {
            let ct = parse_counter_type("P1P1");
            ctx.game.card_mut(token_id).add_counter(&ct, counter_amount);
        }

        ctx.trigger_handler
            .register_active_trigger(ctx.game, token_id);

        ctx.trigger_handler.run_trigger(
            TriggerType::TokenCreated,
            RunParams {
                card: Some(token_id),
                player: Some(player),
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
    } else {
        // Fallback: create inline Incubator token
        ctx.sync_token_art_rng("incubator_c_0_0_a_phyrexian", sa);

        let mut token = Card::new(
            CardId(0),
            "Incubator Token".to_string(),
            player,
            CardTypeLine::parse("Artifact - Incubator"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        token.set_controller(player);
        token.set_is_token(true);

        let token_id = ctx.game.create_card(token);
        ctx.move_card(token_id, ZoneType::Battlefield, player);

        // Add +1/+1 counters
        if counter_amount > 0 {
            let ct = parse_counter_type("P1P1");
            ctx.game.card_mut(token_id).add_counter(&ct, counter_amount);
        }

        ctx.trigger_handler
            .register_active_trigger(ctx.game, token_id);

        ctx.trigger_handler.run_trigger(
            TriggerType::TokenCreated,
            RunParams {
                card: Some(token_id),
                player: Some(player),
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
}
