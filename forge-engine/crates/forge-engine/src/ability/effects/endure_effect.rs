//! Endure — creature endures: either put +1/+1 counters on it, or create
//! a Spirit token with power/toughness equal to the endure amount.
//! Ported from Java's EndureEffect.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Num", 1).max(0);
    if amount < 1 {
        return; // CR 701.63b
    }

    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        ctx.game.card(source).remembered_cards.clone()
    } else {
        return;
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }

        // Option 1: Put +1/+1 counters on the creature
        // (In Java, the player confirms; here we auto-accept for creatures in play)
        let counter_type = super::parse_counter_type("P1P1");
        ctx.game
            .card_mut(card_id)
            .add_counter(&counter_type, amount);
    }
}
