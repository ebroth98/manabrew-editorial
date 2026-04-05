use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ability::spell_ability_effect::get_target_players;
use crate::spellability::SpellAbility;

/// SP$ Discard — target/defined players discard N cards.
///
/// Mirrors Java's `DiscardEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num: usize = sa
        .params
        .as_usize(crate::parsing::keys::NUM_CARDS)
        .unwrap_or(1);

    // Mode$ Random — discard at random (e.g. Hypnotic Specter).
    // Mirrors Java's DiscardEffect which calls Aggregates.random() bypassing the controller.
    // We route through the agent's choose_random_discard so deterministic agents can
    // use their seeded RNG for parity testing.
    let is_random = sa
        .params
        .get(crate::parsing::keys::MODE)
        .map_or(false, |m| m.eq_ignore_ascii_case("Random"));

    for target_player in get_target_players(ctx.game, sa) {
        let hand: Vec<_> = ctx
            .game
            .cards_in_zone(ZoneType::Hand, target_player)
            .to_vec();

        if sa.params.has(crate::parsing::keys::OPTIONAL) {
            let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
            let accepted = ctx.agents[target_player.index()].confirm_action(
                target_player,
                None,
                "Do you want to discard?",
                &[],
                source_name,
                Some(crate::ability::api_type::ApiType::Discard),
            );
            if !accepted {
                continue;
            }
        }

        let to_discard = if is_random {
            ctx.agents[target_player.index()].choose_random_discard(target_player, &hand, num)
        } else {
            ctx.agents[target_player.index()].choose_discard(target_player, &hand, num)
        };

        for card_id in to_discard {
            if ctx.game.card(card_id).zone == ZoneType::Hand {
                super::helpers::discard_with_madness_replacement(
                    ctx.game,
                    ctx.trigger_handler,
                    card_id,
                    target_player,
                    Some(sa),
                );
            }
        }
    }
}
