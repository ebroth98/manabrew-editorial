use super::{parse_param, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1);
    let target = sa
        .params
        .get("Defined")
        .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        .unwrap_or(sa.activating_player);

    // Run DrawCards replacement effects before drawing multiple cards.
    if num > 1 {
        let mut event = ReplacementEvent::DrawCards {
            player: target,
            count: num,
        };
        let result = apply_replacements(ctx.game, &mut event);
        if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
            return;
        }
    }

    if sa.params.contains_key("Optional") {
        let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
        let accepted = ctx.agents[target.index()].confirm_action(
            target,
            None,
            &format!("Do you want to draw {} card(s)?", num),
            &[],
            source_name,
            Some("Draw"),
        );
        if !accepted {
            return;
        }
    }
    // Draw cards one at a time and fire Drawn trigger after each draw.
    // This ensures `drawn_this_turn` is correct for triggers with `Number$ N`
    // (e.g. Sneaky Snacker: "When you draw your third card in a turn...").
    for _ in 0..num {
        if let Some(card_id) = ctx.game.draw_card(target) {
            // Snapshot drawn_this_turn AFTER draw_card increments it.
            // This captures the exact count at draw time for Number$ N matching.
            let drawn_snapshot = ctx.game.player(target).drawn_this_turn;
            ctx.trigger_handler.run_trigger(
                TriggerType::Drawn,
                RunParams {
                    card: Some(card_id),
                    player: Some(target),
                    drawn_this_turn_snapshot: Some(drawn_snapshot),
                    ..Default::default()
                },
                false,
            );
            // Flush/match Drawn triggers immediately so that triggers with
            // Number$ N (e.g. Sneaky Snacker "3rd card") see the correct game
            // state at draw time. Only flush if there's a Number$ Drawn trigger
            // that needs fire-time matching (to avoid disrupting other triggers).
            if ctx.trigger_handler.has_number_drawn_triggers(ctx.game) {
                ctx.trigger_handler.flush_waiting_triggers(ctx.game);
            }
        }
    }
}
