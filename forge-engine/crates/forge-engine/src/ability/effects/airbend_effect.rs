//! Airbend — exile target cards, their owner may cast them for {2}.
//! Ported from Java's AirbendEffect: exiles cards and creates a continuous
//! effect allowing the owner to cast them for an alternate cost of {2}.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `AirbendEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(AirbendEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        if let Some(def) = sa.defined() {
            if def == "Self" {
                vec![source]
            } else {
                ctx.game.card(source).remembered_cards.clone()
            }
        } else {
            vec![source]
        }
    } else {
        return;
    };

    for card_id in targets {
        if ctx.game.card(card_id).zone == ZoneType::None {
            continue;
        }

        // Exile the card
        let old_zone = ctx.game.card(card_id).zone;
        let owner = ctx.game.card(card_id).owner;
        ctx.move_card(card_id, ZoneType::Exile, owner);
        super::emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Exile);

        // Mark the card as castable for {2} from exile (via svar)
        ctx.game
            .card_mut(card_id)
            .set_s_var("AirbendCastable", "MayPlayAltManaCost$2");
    }
}
