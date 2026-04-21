use forge_foundation::ZoneType;

use super::{resolve_defined_player, EffectContext};
use crate::agent::GameLogEvent;
use crate::spellability::SpellAbility;

/// Mirrors Java's `RevealHandEffect.java`.
///
/// `SP$ RevealHand | Defined$ Player`
/// The target player reveals their entire hand to all other players.
/// In the engine this is informational — we notify all agents.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `RevealHandEffect` class extending `SpellAbilityEffect`.
pub struct RevealHandEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for RevealHandEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let target = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    if sa.params.has(crate::parsing::keys::OPTIONAL) {
        let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
        let accepted = ctx.agents[target.index()].confirm_action(
            target,
            None,
            "Do you want to reveal your hand?",
            &[],
            source_name,
            Some(crate::ability::api_type::ApiType::RevealHand),
        );
        if !accepted {
            return;
        }
    }

    let hand = ctx.game.cards_in_zone(ZoneType::Hand, target).to_vec();

    let names: Vec<String> = hand
        .iter()
        .map(|&id| ctx.game.card(id).card_name.clone())
        .collect();
    let msg = format!(
        "Player {} reveals their hand: [{}]",
        target.0,
        names.join(", ")
    );
    for agent in ctx.agents.iter_mut() {
        agent.notify(crate::agent::notification::GameNotification::Event(
            GameLogEvent::rule(msg.clone()).with_player(target),
        ));
    }
    }
}
