//! Cloak effect — like Manifest but with Ward {2}.
//!
//! Ported from Java's `CloakEffect.java`.
//! Cloak: Put card face-down as 2/2 creature with ward {2}.

use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(0) as usize;
    let controller = sa.activating_player;

    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    for pid in players {
        cloak_for_player(ctx, sa, pid, amount);
    }
}

fn cloak_for_player(ctx: &mut EffectContext, sa: &SpellAbility, player: PlayerId, amount: usize) {
    // Default: top N cards of library
    let lib = ctx.game.cards_in_zone(ZoneType::Library, player).to_vec();
    let cards: Vec<CardId> = lib.into_iter().rev().take(amount).collect();

    for card_id in cards {
        let old_zone = ctx.game.card(card_id).zone;

        ctx.game.card_mut(card_id).face_down = true;
        ctx.game.card_mut(card_id).cloaked = true;
        ctx.game.card_mut(card_id).base_power = Some(2);
        ctx.game.card_mut(card_id).base_toughness = Some(2);
        // Ward {2} while cloaked
        ctx.game.card_mut(card_id).pump_keywords.push("Ward:2".to_string());
        ctx.game.card_mut(card_id).controller = player;

        if sa.is_tapped() {
            ctx.game.tap(card_id);
        }

        ctx.game.move_card(card_id, ZoneType::Battlefield, player);
        ctx.trigger_handler.register_active_trigger(ctx.game, card_id);

        if sa.param_is_true("RememberCloaked") {
            if let Some(sid) = sa.source {
                ctx.game.card_mut(sid).add_remembered_card(card_id);
            }
        }

        emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Battlefield);
    }
}
