//! ControlPlayer effect — take control of another player's turn (Mindslaver).
//!
//! Ported 1:1 from Java's `ControlPlayerEffect.java`.
//! You control target player during their next turn. (CR 800.4b)
//! The controlled player's decisions are made by the controller.

use super::EffectContext;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller_def = sa.params.get(keys::CONTROLLER).map(|s| s.to_string()).unwrap_or_else(|| "You".to_string());
    let controller = super::resolve_defined_players(&controller_def, sa.activating_player, ctx.game)
        .into_iter()
        .next()
        .unwrap_or(sa.activating_player);

    let targets = if let Some(pid) = sa.target_chosen.target_player {
        vec![pid]
    } else if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, sa.activating_player, ctx.game)
    } else {
        vec![ctx.game.opponent_of(sa.activating_player)]
    };

    for target_player in targets {
        // Set the controlled_by field on the target player
        // This will be checked by the game loop to route decisions
        // through the controller's agent instead of the target's agent
        ctx.game.player_mut(target_player).controlled_by = Some(controller);

        // Register a cleanup to remove control at end of next turn
        // Java uses addUntil on the cleanup step — we use a delayed trigger
        ctx.trigger_handler.register_delayed_trigger(
            crate::trigger::handler::DelayedTrigger {
                mode: crate::event::TriggerType::Phase,
                trigger_mode: crate::trigger::TriggerMode::Always,
                execute_svar: "ControlPlayerCleanup".to_string(),
                controller,
                source_card: sa.source.unwrap_or(crate::ids::CardId(0)),
                target_card: None,
                remembered_amount: target_player.0 as i32,
            },
        );
    }
}
