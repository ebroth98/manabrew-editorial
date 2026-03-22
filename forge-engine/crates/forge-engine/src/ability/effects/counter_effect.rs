use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::parsing::keys;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

/// SP$ Counter — remove a targeted spell from the stack and put it into
/// its owner's graveyard (or exile, per Destination$ if present).
///
/// Supports `UnlessCost$` — if present, the targeted spell's controller is
/// prompted to pay; if they accept, the spell is NOT countered.
/// Mirrors Java's `CounterEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let entry_id = match sa.target_chosen.target_stack_entry {
        Some(id) => id,
        None => return, // no target chosen
    };

    // Determine destination (default: graveyard).
    let dest_zone = sa
        .params
        .get(keys::DESTINATION)
        .and_then(|d| super::parse_zone_type(d))
        .unwrap_or(ZoneType::Graveyard);

    // UnlessCost$: ask the targeted spell's controller whether they want to pay.
    // If they pay, the spell is not countered (Ward, Mana Leak, etc.).
    if let Some(unless_cost) = sa.params.get(keys::UNLESS_COST) {
        if let Some(entry) = ctx.game.stack.find_by_id(entry_id) {
            let spell_controller = entry.spell_ability.activating_player;
            let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());
            let message = format!("Pay {} to avoid counter?", unless_cost);
            let paid = ctx.agents[spell_controller.index()].confirm_action(
                spell_controller,
                None,
                &message,
                &[],
                source_name.as_deref(),
                None, // Ward is not an ApiType
            );
            if paid {
                return; // opponent chose to pay — spell is not countered
            }
        }
    }

    // Check if the spell has a "can't be countered" replacement effect.
    // Find the source card of the targeted stack entry.
    if let Some(entry) = ctx.game.stack.find_by_id(entry_id) {
        if let Some(source_card) = entry.spell_ability.source {
            let mut event = ReplacementEvent::Counter { card: source_card };
            let result = apply_replacements(ctx.game, &mut event);
            if result == ReplacementResult::Replaced {
                return;
            }
        }
    }

    // Remove from stack
    if let Some(entry) = ctx.game.stack.remove_by_id(entry_id) {
        let countered_sa = &entry.spell_ability;
        if let Some(source_card) = countered_sa.source {
            // Only move if the card is still "virtual" (on the stack, zone = None is fine)
            // — it was removed from hand when cast; move it to dest zone now.
            let owner = ctx.game.card(source_card).owner;

            // Remember parameters if needed
            if sa.params.has(keys::REMEMBER_COUNTERED) {
                ctx.game
                    .card_mut(sa.source.unwrap())
                    .add_remembered_card(source_card);
            }
            if sa.params.has(keys::REMEMBER_COUNTERED_CMC) {
                // Store CMC value
                let cmc = ctx.game.card(source_card).mana_cost.cmc();
                ctx.game
                    .card_mut(sa.source.unwrap())
                    .add_remembered_cmc(cmc);
            }

            ctx.game.move_card(source_card, dest_zone, owner);
            emit_zone_trigger(ctx.trigger_handler, source_card, ZoneType::Stack, dest_zone);

            // Fire Countered trigger
            ctx.trigger_handler.run_trigger(
                TriggerType::Countered,
                RunParams {
                    card: Some(source_card),
                    spell_ability: Some(countered_sa.clone()),
                    cause: Some(sa.clone()),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
