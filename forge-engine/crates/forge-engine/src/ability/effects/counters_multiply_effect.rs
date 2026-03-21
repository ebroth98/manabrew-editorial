//! CountersMultiply effect — double (or multiply) counters on a permanent.
//!
//! Ported from Java's `CountersMultiplyEffect.java`.
//! Double the number of each type of counter on target permanent.

use super::{parse_counter_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use forge_foundation::ZoneType;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let multiplier = super::resolve_numeric_svar(ctx.game, sa, "Multiplier", 2).max(0);
    let counter_type_filter = sa.params.get_cloned(keys::COUNTER_TYPE);

    let targets: Vec<crate::ids::CardId> = if sa.uses_targeting() {
        sa.target_chosen.target_card.into_iter().collect()
    } else {
        sa.source.into_iter().collect()
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield { continue; }

        if let Some(ref ct_str) = counter_type_filter {
            // Multiply specific counter type
            let ct = parse_counter_type(ct_str);
            let current = *ctx.game.card(card_id).counters.get(&ct).unwrap_or(&0);
            let to_add = current * (multiplier - 1);
            if to_add > 0 {
                ctx.game.card_mut(card_id).add_counter(&ct, to_add);
            }
        } else {
            // Multiply ALL counter types
            let counters: Vec<(crate::card::CounterType, i32)> = ctx.game.card(card_id)
                .counters.iter().map(|(k, &v)| (k.clone(), v)).collect();
            for (ct, current) in counters {
                let to_add = current * (multiplier - 1);
                if to_add > 0 {
                    ctx.game.card_mut(card_id).add_counter(&ct, to_add);
                }
            }
        }
    }
}
