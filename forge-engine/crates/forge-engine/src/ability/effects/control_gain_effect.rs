use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// End-of-turn revert for control gain. Mirrors the `GameCommand.run()` in Java
/// `ControlGainEffect` that restores the original controller and removes granted
/// keywords when the effect duration expires.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId) {
    if game.card(card_id).zone != ZoneType::Battlefield {
        return;
    }
    // Restore original controller if an EOT controller was set
    if let Some(original) = game.card(card_id).original_controller_eot {
        game.change_controller(card_id, original);
        game.card_mut(card_id).set_original_controller_eot(None);
    }
    // Clear any granted keywords that were part of the control-gain effect
    game.card_mut(card_id).clear_granted_keywords();
}

/// SP$ ControlGain — gain control of target permanent until end of turn or permanently.
///
/// Mirrors Java's `ControlGainEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let target_card = match sa.target_chosen.target_card {
        Some(c) => c,
        None => return,
    };

    // Verify target is still on the battlefield
    if ctx.game.card(target_card).zone != ZoneType::Battlefield {
        return;
    }

    let new_controller = sa.activating_player;

    // Check if the card can be controlled by the new controller
    if !ctx
        .game
        .card(target_card)
        .can_be_controlled_by(new_controller)
    {
        return;
    }

    // Change controller
    let old_controller = ctx.game.card(target_card).controller;
    ctx.game.change_controller(target_card, new_controller);

    // Fire ChangesController trigger (mirrors Java GameAction.doChangeController)
    if old_controller != new_controller {
        ctx.trigger_handler.run_trigger(
            TriggerType::ChangesController,
            RunParams {
                card: Some(target_card),
                player: Some(new_controller),
                original_controller: Some(old_controller),
                ..Default::default()
            },
            false,
        );
    }

    // Schedule controller return at end of turn if LoseControl$ EOT
    if sa
        .params
        .get(keys::LOSE_CONTROL)
        .map(|v| v == "EOT")
        .unwrap_or(false)
    {
        ctx.game
            .card_mut(target_card)
            .set_original_controller_eot(Some(old_controller));
    }

    // Handle Untap parameter
    if sa.params.has(keys::UNTAP) {
        ctx.game.untap(target_card);
    }

    // Handle AddKWs parameter (add keywords)
    if let Some(kws_str) = sa.params.get(keys::ADD_KWS) {
        let keywords: Vec<String> = kws_str.split(" & ").map(|s| s.to_string()).collect();
        for kw in keywords {
            ctx.game.card_mut(target_card).add_granted_keyword(&kw);
        }
    }
}
