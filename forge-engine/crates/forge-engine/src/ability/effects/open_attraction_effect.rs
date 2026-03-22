//! OpenAttraction — open an attraction from the attraction deck (Unfinity).
//! Ported from Java's OpenAttractionEffect: moves top card of attraction
//! deck to battlefield.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source = match sa.source {
        Some(s) => s,
        None => return,
    };

    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(1);

    let players = if let Some(def) = sa.params.get(keys::DEFINED) {
        super::resolve_defined_players(def, sa.activating_player, ctx.game)
    } else {
        vec![sa.activating_player]
    };

    for player_id in players {
        if ctx.game.player(player_id).has_lost {
            continue;
        }

        for _ in 0..amount {
            // Find first card in AttractionDeck zone for this player
            let attraction = ctx
                .game
                .cards
                .iter()
                .find(|c| c.zone == ZoneType::Sideboard && c.owner == player_id && c.type_line.subtypes.iter().any(|s| s.eq_ignore_ascii_case("Attraction")))
                .map(|c| c.id);

            if let Some(card_id) = attraction {
                let old_zone = ctx.game.card(card_id).zone;
                ctx.game.move_card(card_id, ZoneType::Battlefield, player_id);
                super::emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Battlefield);

                if sa.param_is_true(keys::REMEMBER) {
                    ctx.game.card_mut(source).add_remembered_card(card_id);
                }
            }
        }
    }
}
