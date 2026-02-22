use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// SP$ Counter — remove a targeted spell from the stack and put it into
/// its owner's graveyard (or exile, per Destination$ if present).
///
/// Mirrors Java's `CounterEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let entry_id = match sa.target_chosen.target_stack_entry {
        Some(id) => id,
        None => return, // no target chosen
    };

    // Determine destination (default: graveyard).
    let dest_zone = sa
        .params
        .get("Destination")
        .and_then(|d| super::parse_zone_type(d))
        .unwrap_or(ZoneType::Graveyard);

    // Remove from stack
    if let Some(entry) = ctx.game.stack.remove_by_id(entry_id) {
        let countered_sa = &entry.spell_ability;
        if let Some(source_card) = countered_sa.source {
            // Only move if the card is still "virtual" (on the stack, zone = None is fine)
            // — it was removed from hand when cast; move it to dest zone now.
            let owner = ctx.game.card(source_card).owner;
            
            // Remember parameters if needed
            if sa.params.contains_key("RememberCountered") {
                ctx.game.card_mut(sa.source.unwrap()).add_remembered_card(source_card);
            }
            if sa.params.contains_key("RememberCounteredCMC") {
                // Store CMC value
                let cmc = ctx.game.card(source_card).mana_cost.cmc();
                ctx.game.card_mut(sa.source.unwrap()).add_remembered_cmc(cmc);
            }
            
            ctx.game.move_card(source_card, dest_zone, owner);
            
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