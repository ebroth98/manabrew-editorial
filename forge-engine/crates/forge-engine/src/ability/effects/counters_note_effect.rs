//! CountersNote effect — note the number of counters on a permanent.
//!
//! Ported from Java's `CountersNoteEffect.java`.
//! Store counter amounts for later use (e.g. "noted P1P1 counters").

use super::{parse_counter_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else { return };
    let counter_type_str = sa
        .params
        .get(keys::COUNTER_TYPE)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "P1P1".to_string());
    let counter_type = parse_counter_type(&counter_type_str);

    let targets: Vec<crate::ids::CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card.into_iter().collect()
    } else {
        sa.source.into_iter().collect()
    };

    for card_id in targets {
        let count = *ctx
            .game
            .card(card_id)
            .counters
            .get(&counter_type)
            .unwrap_or(&0);
        // Store noted value in source's remembered_cmc (used by WithNotedCounters$)
        ctx.game.card_mut(source_id).add_remembered_cmc(count);
    }
}
