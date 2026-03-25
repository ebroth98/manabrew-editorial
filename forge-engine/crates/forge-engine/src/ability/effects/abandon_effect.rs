//! Abandon — leave a game in a multiplayer match.
//! Ported from Java's AbandonEffect: sets player as lost.

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use forge_foundation::ZoneType;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else {
        return;
    };
    let controller = ctx.game.card(source_id).controller;

    if sa.param_is_true(keys::OPTIONAL) {
        let source_name = ctx.game.card(source_id).card_name.clone();
        let confirmed = ctx.agents[controller.index()].confirm_action(
            controller,
            None,
            &format!("Would you like to abandon {source_name}?"),
            &[],
            Some(&source_name),
            None,
        );
        if !confirmed {
            return;
        }
    }

    if sa.params.get("RememberAbandoned").is_some() {
        ctx.game.card_mut(source_id).add_remembered(source_id);
    }

    if ctx.game.card(source_id).zone == ZoneType::Command {
        ctx.game
            .move_card(source_id, ZoneType::SchemeDeck, controller);
    }

    ctx.trigger_handler.run_trigger(
        TriggerType::Abandoned,
        RunParams {
            card: Some(source_id),
            ..Default::default()
        },
        false,
    );
}
