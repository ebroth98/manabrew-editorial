//! Airbend — exile target cards, their owner may cast them for {2}.
//! Ported from Java's AirbendEffect: exiles cards and creates a continuous
//! effect allowing the owner to cast them for an alternate cost of {2}.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        if let Some(def) = sa.params.get(keys::DEFINED) {
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
        ctx.game.move_card(card_id, ZoneType::Exile, owner);
        super::emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Exile);

        // Mark the card as castable for {2} from exile (via svar)
        ctx.game.card_mut(card_id).svars.insert(
            "AirbendCastable".to_string(),
            "MayPlayAltManaCost$2".to_string(),
        );
    }
}
