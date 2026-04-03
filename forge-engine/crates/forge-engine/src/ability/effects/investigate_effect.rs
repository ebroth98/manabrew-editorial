//! Investigate effect — create Clue artifact tokens.
//!
//! Ported from Java's `InvestigateEffect.java`.
//! Investigate: Create a colorless Clue artifact token with
//! "{2}, Sacrifice this artifact: Draw a card."

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use super::{emit_zone_trigger, EffectContext};
use crate::card::card_zone_table::CardZoneTable;
use crate::card::Card;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0) as usize;
    let controller = sa.activating_player;

    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    let mut created_tokens: Vec<CardId> = Vec::new();
    for _ in 0..amount {
        for &pid in &players {
            if let Some(tid) = create_clue_token(ctx, sa, pid) {
                created_tokens.push(tid);
            }
        }
    }

    // Fire ChangesZoneAll for the batch of tokens entering the battlefield.
    // Needed for triggers like Woodland Champion (Mode$ ChangesZoneAll).
    if !created_tokens.is_empty() {
        let mut table = CardZoneTable::default();
        for &tid in &created_tokens {
            table.put(Some(ZoneType::None), Some(ZoneType::Battlefield), tid);
        }
        table.trigger_changes_zone_all(ctx.trigger_handler, ctx.game, Some(sa));
    }
}

/// Create a single Clue artifact token. Returns the created token ID.
fn create_clue_token(ctx: &mut EffectContext, sa: &SpellAbility, player: crate::ids::PlayerId) -> Option<CardId> {
    let token_id;
    // Try to use the registered token template first
    if let Some(template) = ctx.token_templates.get("c_a_clue_draw").cloned() {
        // RNG sync
        ctx.rng.next_int(1);
        ctx.rng.next_int(1);

        let mut token = template;
        token.set_owner(player);
        token.set_controller(player);
        token.set_is_token(true);

        token_id = ctx.game.create_card(token);
        ctx.move_card(token_id, ZoneType::Battlefield, player);
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
        // Fallback: create inline Clue token
        ctx.rng.next_int(1);
        ctx.rng.next_int(1);

        let mut token = Card::new(
            CardId(0),
            "Clue Token".to_string(),
            player,
            CardTypeLine::parse("Artifact - Clue"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        token.set_controller(player);
        token.set_is_token(true);

        token_id = ctx.game.create_card(token);
        ctx.move_card(token_id, ZoneType::Battlefield, player);
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

    // RememberInvestigatingPlayers$
    if sa.param_is_true(keys::REMEMBER_INVESTIGATING_PLAYERS) {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).add_remembered_player(player);
        }
    }

    Some(token_id)
}
